import { isCancellationError } from "../../../../base/common/cancellation.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableStore, type IDisposable } from "../../../../base/common/lifecycle.js";
import { materializeTextMateGrammarCatalog, TextMateGrammarCatalogModel, type TextMateGrammarCatalog, type TextMateGrammarCatalogSource } from "./textMateGrammarCatalog.js";
import { TextMateGrammarRegistry, type TextMateGrammarDefinition, type TextMateGrammarRegistration, type TextMateGrammarRegistrySnapshot } from "./textMateGrammarRegistry.js";

export interface TextMateGrammarCatalogFailure {
  readonly revision: number;
  readonly error: unknown;
}

export interface ITextMateGrammarService extends TextMateGrammarCatalogSource {
  readonly onDidFailCatalog: Event<TextMateGrammarCatalogFailure>;
  registerGrammar(definition: TextMateGrammarDefinition): IDisposable;
  registerGrammars(definitions: readonly TextMateGrammarDefinition[]): TextMateGrammarRegistration;
  prepareGrammars(registration: TextMateGrammarRegistration, definitions: readonly TextMateGrammarDefinition[]): Promise<PreparedTextMateGrammars>;
  whenReady(): Promise<TextMateGrammarCatalog>;
}

/** A fully materialized grammar set whose commit cannot perform resource I/O. */
export interface PreparedTextMateGrammars {
  commit(): TextMateGrammarCatalog;
}

/** Owns TextMate contributions and publishes their latest complete transferable catalog. */
export class TextMateGrammarService extends DisposableOwner implements ITextMateGrammarService {
  private readonly registry = this.own(new TextMateGrammarRegistry());
  private readonly catalogs = this.own(new TextMateGrammarCatalogModel());
  private readonly failureEmitter = this.own(new Emitter<TextMateGrammarCatalogFailure>());
  private readonly registrations = this.own(new DisposableStore());
  private materialization: Promise<TextMateGrammarCatalog> = Promise.resolve(this.catalogs.currentCatalog);
  private materializationController: AbortController | undefined;
  private preparedCatalog: TextMateGrammarCatalog | undefined;
  private preparedBaseRevision: number | undefined;
  private disposed = false;

  readonly onDidChangeCatalog = this.catalogs.onDidChangeCatalog;
  readonly onDidFailCatalog = this.failureEmitter.event;

  constructor() {
    super();
    this.own(this.registry.onDidChange(snapshot => this.scheduleMaterialization(snapshot)));
    this.defer(() => {
      this.disposed = true;
      this.materializationController?.abort(new Error("TextMate grammar service disposed"));
      this.materializationController = undefined;
    });
  }

  get currentCatalog(): TextMateGrammarCatalog {
    this.ensureAlive();
    return this.catalogs.currentCatalog;
  }

  registerGrammar(definition: TextMateGrammarDefinition): IDisposable {
    this.ensureAlive();
    return this.registrations.add(this.registry.register(definition));
  }

  registerGrammars(definitions: readonly TextMateGrammarDefinition[]): TextMateGrammarRegistration {
    this.ensureAlive();
    return this.registrations.add(this.registry.registerMany(definitions));
  }

  async prepareGrammars(registration: TextMateGrammarRegistration, definitions: readonly TextMateGrammarDefinition[]): Promise<PreparedTextMateGrammars> {
    this.ensureAlive();
    if (!registration || typeof registration.prepare !== "function" || typeof registration.owns !== "function" || !registration.owns(this.registry.currentSnapshot)) throw new TypeError("TextMate grammar preparation requires a registration owned by this service");
    const baseRegistrySnapshot = registration.currentSnapshot;
    const baseCatalogRevision = this.catalogs.currentCatalog.revision;
    const preparedReplacement = registration.prepare(definitions);
    const catalog = await materializeTextMateGrammarCatalog(preparedReplacement.snapshot, preparedReplacement.snapshot.revision, new AbortController().signal);
    if (!registration.owns(baseRegistrySnapshot)) throw new Error("TextMate grammar registry changed during preparation");
    if (this.catalogs.currentCatalog.revision !== baseCatalogRevision) throw new Error("TextMate grammar catalog changed during preparation");
    let committed = false;
    return Object.freeze({
      commit: () => {
        if (committed) throw new ReferenceError("Prepared TextMate grammars are already committed");
        this.ensureAlive();
        committed = true;
        if (!registration.owns(baseRegistrySnapshot)) throw new Error("TextMate grammar registry changed after preparation");
        if (this.catalogs.currentCatalog.revision !== baseCatalogRevision) throw new Error("TextMate grammar catalog changed after preparation");
        const committedCatalog = Object.freeze({ revision: preparedReplacement.snapshot.revision, grammars: catalog.grammars });
        this.preparedCatalog = committedCatalog;
        this.preparedBaseRevision = baseCatalogRevision;
        try { preparedReplacement.commit(); }
        catch (error) {
          this.preparedCatalog = undefined;
          this.preparedBaseRevision = undefined;
          throw error;
        }
        return this.catalogs.currentCatalog;
      },
    });
  }

  whenReady(): Promise<TextMateGrammarCatalog> {
    this.ensureAlive();
    return this.materialization;
  }

  private scheduleMaterialization(snapshot: TextMateGrammarRegistrySnapshot): void {
    this.materializationController?.abort(new Error("Superseded TextMate grammar catalog revision"));
    const prepared = this.preparedCatalog;
    if (prepared && this.preparedBaseRevision === this.catalogs.currentCatalog.revision) {
      this.preparedCatalog = undefined;
      this.preparedBaseRevision = undefined;
      this.materializationController = undefined;
      const committed = prepared.revision === snapshot.revision ? prepared : Object.freeze({ revision: snapshot.revision, grammars: prepared.grammars });
      this.catalogs.replace(committed);
      this.materialization = Promise.resolve(this.catalogs.currentCatalog);
      return;
    }
    const controller = new AbortController();
    this.preparedCatalog = undefined;
    this.preparedBaseRevision = undefined;
    this.materializationController = controller;
    const operation = materializeTextMateGrammarCatalog(snapshot, snapshot.revision, controller.signal).then(catalog => {
      if (this.disposed || this.materializationController !== controller) return catalog;
      this.catalogs.replace(catalog);
      return this.catalogs.currentCatalog;
    });
    operation.catch(error => {
      if (this.disposed || this.materializationController !== controller || isCancellationError(error)) return;
      this.failureEmitter.fire(Object.freeze({ revision: snapshot.revision, error }));
    });
    this.materialization = operation;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("TextMateGrammarService is already disposed");
  }
}
