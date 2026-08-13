import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type VersionedLanguageResultStore } from "../../../common/languages/languageResultStore.js";
import { type LanguageDiagnostic, type LanguageDiagnosticResult } from "../../../common/languages/languageResults.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";
import { type URI } from "../../../../base/common/uri.js";
import { type LanguageDiagnosticsPublisher, type LanguageDiagnosticsSource } from "../../../common/services/languageDiagnosticsService.js";

/**
 * Projects current-version diagnostics into generic text decorations.
 *
 * The bridge observes but does not own the result store or text model. It owns
 * its projected collection and clears it whenever the store loses its result.
 */
export class LanguageDiagnosticDecorationBridge extends DisposableOwner {
  readonly decorations: TextDecorationCollection<LanguageDiagnostic>;

  constructor(private readonly store: VersionedLanguageResultStore<LanguageDiagnosticResult>, private readonly externalSource?: LanguageDiagnosticsSource, private readonly resource?: URI) {
    super();
    this.decorations = this.own(new TextDecorationCollection(store.textModel));
    try {
      this.own(store.onDidChange(() => this.synchronize()));
      if (externalSource) this.own(externalSource.onDidChangeDiagnostics(resource => {
        if (resource.toString() === this.resource?.toString()) this.synchronize();
      }));
      if (externalSource) this.own(store.textModel.onDidChange(() => this.synchronize()));
      this.synchronize();
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private synchronize(): void {
    const localDiagnostics = this.store.result?.value.diagnostics ?? [];
    const external = this.resource ? this.externalSource?.getDiagnostics(this.resource) : undefined;
    const externalDiagnostics = external?.revision === this.store.textModel.version ? external.diagnostics : [];
    const diagnostics = deduplicateDiagnostics([...localDiagnostics, ...externalDiagnostics]);
    this.decorations.replaceAll(diagnostics.map(diagnostic => ({
      range: diagnostic.range,
      stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
      metadata: diagnostic,
    })));
  }
}

/** Publishes the local syntax worker's current revision into the shared diagnostic repository. */
export class LanguageDiagnosticPublisherBridge extends DisposableOwner {
  constructor(private readonly store: VersionedLanguageResultStore<LanguageDiagnosticResult>, private readonly publisher: LanguageDiagnosticsPublisher) {
    super();
    this.own(publisher);
    this.own(store.onDidChange(() => this.synchronize()));
    this.synchronize();
  }

  private synchronize(): void {
    const result = this.store.result;
    this.publisher.update(result?.modelVersion ?? this.store.textModel.version, result?.value.diagnostics ?? []);
  }
}

function deduplicateDiagnostics(diagnostics: readonly LanguageDiagnostic[]): readonly LanguageDiagnostic[] {
  const seen = new Set<string>();
  return diagnostics.filter(diagnostic => {
    const range = diagnostic.range;
    const key = `${range.start.lineIndex}:${range.start.columnIndex}:${range.end.lineIndex}:${range.end.columnIndex}:${diagnostic.severity}:${diagnostic.message}:${diagnostic.source ?? ""}:${diagnostic.code ?? ""}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
