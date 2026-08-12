import type { SyntaxAnalyzeResult } from "../../../../../generated/app-server/types.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorFoldingRange, EditorFoldingRangeSource } from "../../contrib/folding/browser/foldingRanges.js";
import { type TextSnapshot } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { RustSyntaxFactsService, syntaxLanguageForAsterLanguage } from "./rustSyntaxFactsService.js";

/** Keeps Aster's parser-derived folding ranges synchronized with the Rust syntax endpoint. */
export class RustSyntaxFoldingService extends DisposableOwner {
  private readonly supported: boolean;
  private generation = 0;
  private disposed = false;
  private _ranges: readonly EditorFoldingRange[] = Object.freeze([]);

  get ranges(): readonly EditorFoldingRange[] {
    return this._ranges;
  }

  constructor(
    private readonly model: TextModel,
    private readonly languageId: string,
    private readonly facts: RustSyntaxFactsService,
    private readonly onDidChange: () => void,
    private readonly onError: (error: unknown) => void,
  ) {
    super();
    this.defer(() => {
      this.disposed = true;
      this.generation += 1;
    });
    this.supported = syntaxLanguageForAsterLanguage(languageId) !== undefined;
    if (!this.supported) return;
    this.own(model.onDidChange(() => this.refresh()));
    this.request();
  }

  private refresh(): void {
    this.generation += 1;
    if (this._ranges.length > 0) {
      this._ranges = Object.freeze([]);
      this.onDidChange();
    }
    this.request();
  }

  private request(): void {
    if (!this.supported) return;
    const generation = ++this.generation;
    const snapshot = this.model.createSnapshot();
    queueMicrotask(() => {
      void this.analyze(generation, snapshot);
    });
  }

  private async analyze(generation: number, snapshot: TextSnapshot): Promise<void> {
    try {
      const result = await this.facts.analyze(this.languageId, snapshot, new AbortController().signal);
      if (this.disposed || this.generation !== generation) return;
      this._ranges = result ? projectRustSyntaxFoldingRanges(result, snapshot.version) : Object.freeze([]);
      this.onDidChange();
    } catch (error) {
      if (!this.disposed && this.generation === generation) this.onError(error);
    }
  }
}

export function projectRustSyntaxFoldingRanges(result: Pick<SyntaxAnalyzeResult, "revision" | "foldingRanges">, expectedRevision: number): readonly EditorFoldingRange[] {
  if (result.revision !== expectedRevision || !Array.isArray(result.foldingRanges)) return Object.freeze([]);
  const ranges: EditorFoldingRange[] = [];
  for (const foldingRange of result.foldingRanges) {
    const startLineIndex = foldingRange?.range?.start?.lineIndex;
    const endLineIndex = foldingRange?.range?.end?.lineIndex;
    if (!Number.isSafeInteger(startLineIndex) || !Number.isSafeInteger(endLineIndex) || startLineIndex < 0 || endLineIndex <= startLineIndex) continue;
    ranges.push(Object.freeze({
      startLineIndex,
      endLineIndex,
      collapsed: false,
      source: EditorFoldingRangeSource.Provider,
    }));
  }
  return Object.freeze(ranges);
}
