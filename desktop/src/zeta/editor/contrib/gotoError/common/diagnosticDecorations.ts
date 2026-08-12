import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type VersionedLanguageResultStore } from "../../../common/languages/languageResultStore.js";
import { type LanguageDiagnostic, type LanguageDiagnosticResult } from "../../../common/languages/languageResults.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";

/**
 * Projects current-version diagnostics into generic text decorations.
 *
 * The bridge observes but does not own the result store or text model. It owns
 * its projected collection and clears it whenever the store loses its result.
 */
export class LanguageDiagnosticDecorationBridge extends DisposableOwner {
  readonly decorations: TextDecorationCollection<LanguageDiagnostic>;

  constructor(private readonly store: VersionedLanguageResultStore<LanguageDiagnosticResult>) {
    super();
    this.decorations = this.own(new TextDecorationCollection(store.textModel));
    try {
      this.own(store.onDidChange(() => this.synchronize()));
      this.synchronize();
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private synchronize(): void {
    const diagnostics = this.store.result?.value.diagnostics ?? [];
    this.decorations.replaceAll(diagnostics.map(diagnostic => ({
      range: diagnostic.range,
      stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
      metadata: diagnostic,
    })));
  }
}
