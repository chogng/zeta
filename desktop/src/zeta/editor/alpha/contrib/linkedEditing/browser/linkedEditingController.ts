import "./media/linkedEditing.css";
import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextRange, type TextModelChange } from "../../../common/core/text.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";
import { type LinkedEditingService } from "../common/linkedEditing.js";

/** Synchronizes provider-declared linked ranges through ordinary model transactions. */
export class AlphaLinkedEditingController extends DisposableOwner {
  private readonly ranges = this.own(new ResettableDisposableGroup());
  private trackedRanges: readonly { readonly range: TrackedRange; readonly lastStartOffset: number; readonly lastEndOffset: number }[] = [];
  private active = false;
  private applying = false;
  private request: AbortController | undefined;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: AlphaEditorViewport, private readonly selections: EditorSelectionController, private readonly service: LinkedEditingService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Alpha linked editing failed", error)) {
    super();
    if (viewport.textModel !== selections.textModel || service.textModel !== selections.textModel) throw new TypeError("Alpha linked editing dependencies must share a text model");
    this.own(addDisposableListener(input, "keydown", event => { if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== "l") return; stopEvent(event); void this.activate(); }, true));
    this.own(addDisposableListener(input, "keydown", event => { if (event.key !== "Escape" || !this.active) return; stopEvent(event); this.clear(); }, true));
    this.own(viewport.textModel.onDidChange(change => this.acceptChange(change)));
    this.defer(() => this.request?.abort());
  }

  private async activate(): Promise<void> {
    this.request?.abort();
    const request = this.request = new AbortController();
    try {
      const result = await this.service.provideLinkedEditingRanges(this.languageId, this.selections.selections.primary.range, request.signal);
      if (request.signal.aborted || !result || result.ranges.length < 2) { this.clear(); return; }
      this.ranges.clear();
      const tracked = result.ranges.map(range => {
        const trackedRange = this.ranges.adopt(this.viewport.textModel.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges), candidate => candidate.dispose());
        return { range: trackedRange, lastStartOffset: this.viewport.textModel.offsetAt(range.start), lastEndOffset: this.viewport.textModel.offsetAt(range.end) };
      });
      this.trackedRanges = tracked;
      this.active = true;
      this.viewport.element.classList.add("linked-editing-active");
      this.viewport.announceAccessibilityStatus(`${result.ranges.length} linked editing ranges active`);
    } catch (error) { if (!request.signal.aborted) this.onError(error); }
  }

  private acceptChange(change: TextModelChange): void {
    if (!this.active || this.applying || change.changes.length !== 1) return;
    const changeItem = change.changes[0]!;
    const changeStartOffset = changeItem.rangeOffset;
    const changeEndOffset = changeStartOffset + changeItem.rangeLength;
    const source = this.trackedRanges.find(candidate => candidate.lastStartOffset <= changeStartOffset && changeEndOffset <= candidate.lastEndOffset);
    if (!source) return;
    const relativeStartOffset = changeStartOffset - source.lastStartOffset;
    const relativeEndOffset = relativeStartOffset + changeItem.rangeLength;
    const edits = this.trackedRanges
      .filter(candidate => candidate !== source)
      .map(candidate => {
        const targetStartOffset = this.viewport.textModel.offsetAt(candidate.range.range.start);
        const targetEndOffset = this.viewport.textModel.offsetAt(candidate.range.range.end);
        const startOffset = targetStartOffset + relativeStartOffset;
        const endOffset = targetStartOffset + relativeEndOffset;
        if (startOffset < targetStartOffset || endOffset > targetEndOffset) return undefined;
        return { range: TextRange.from(this.viewport.textModel.positionAt(startOffset), this.viewport.textModel.positionAt(endOffset)), text: changeItem.text };
      })
      .filter((edit): edit is { readonly range: TextRange; readonly text: string } => edit !== undefined)
      .sort((left, right) => left.range.start.compareTo(right.range.start));
    const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, edits);
    if (!command) return;
    this.applying = true;
    try { this.selections.execute(command); } finally { this.applying = false; }
    this.refreshOffsets();
  }

  private refreshOffsets(): void {
    this.trackedRanges = this.trackedRanges.map(candidate => ({ range: candidate.range, lastStartOffset: this.viewport.textModel.offsetAt(candidate.range.range.start), lastEndOffset: this.viewport.textModel.offsetAt(candidate.range.range.end) }));
  }

  private clear(): void { this.request?.abort(); this.request = undefined; this.ranges.clear(); this.trackedRanges = []; this.active = false; this.viewport.element.classList.remove("linked-editing-active"); }
}
