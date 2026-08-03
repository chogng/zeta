import { isCancellationError } from "../../../../base/common/cancellation.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableStore, type IDisposable } from "../../../../base/common/lifecycle.js";
import { materializeTextMateGrammarCatalog, TextMateGrammarCatalogModel, type TextMateGrammarCatalog, type TextMateGrammarCatalogSource } from "./textMateGrammarCatalog.js";
import { TextMateGrammarRegistry, type TextMateGrammarDefinition, type TextMateGrammarRegistrySnapshot } from "./textMateGrammarRegistry.js";

export interface TextMateGrammarCatalogFailure {
  readonly revision: number;
  readonly error: unknown;
}

export interface ITextMateGrammarService extends TextMateGrammarCatalogSource {
  readonly onDidFailCatalog: Event<TextMateGrammarCatalogFailure>;
  registerGrammar(definition: TextMateGrammarDefinition): IDisposable;
  whenReady(): Promise<TextMateGrammarCatalog>;
}

/** Owns TextMate contributions and publishes their latest complete transferable catalog. */
export class TextMateGrammarService extends DisposableOwner implements ITextMateGrammarService {
  private readonly registry = this.own(new TextMateGrammarRegistry());
  private readonly catalogs = this.own(new TextMateGrammarCatalogModel());
  private readonly failureEmitter = this.own(new Emitter<TextMateGrammarCatalogFailure>());
  private readonly registrations = this.own(new DisposableStore());
  private materialization: Promise<TextMateGrammarCatalog> = Promise.resolve(this.catalogs.currentCatalog);
  private materializationController: AbortController | undefined;
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

  whenReady(): Promise<TextMateGrammarCatalog> {
    this.ensureAlive();
    return this.materialization;
  }

  private scheduleMaterialization(snapshot: TextMateGrammarRegistrySnapshot): void {
    this.materializationController?.abort(new Error("Superseded TextMate grammar catalog revision"));
    const controller = new AbortController();
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
