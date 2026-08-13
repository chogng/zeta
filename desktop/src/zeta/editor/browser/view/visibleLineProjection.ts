import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorLineVisibilitySource, EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorViewportLineSource } from "../../common/viewLayout/editorViewportModel.js";
import { VisualLineProjection } from "./visualLineProjection.js";

/** Filters Aster's wrapped visual rows through an optional logical-line visibility source. */
export class VisibleLineProjection extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<void>());
  private projectionRevision = 0;
  private currentProjection: EditorVisualLineProjection;

  readonly onDidChange: Event<void> = this.changeEmitter.event;
  readonly lineSource: EditorViewportLineSource;

  constructor(private readonly source: VisualLineProjection, private readonly visibility: EditorLineVisibilitySource | undefined) {
    super();
    this.currentProjection = this.createProjection();
    const projection = this;
    this.lineSource = Object.freeze({
      get lineCount(): number {
        return projection.currentProjection.visualLineCount;
      },
      onDidChange: this.onDidChange,
    });
    this.own(source.onDidChange(() => this.rebuild()));
    if (visibility) this.own(visibility.onDidChange(() => this.rebuild()));
  }

  get projection(): EditorVisualLineProjection {
    return this.currentProjection;
  }

  get revision(): number {
    return this.projectionRevision;
  }

  ensureCurrent(): EditorVisualLineProjection {
    this.source.ensureCurrent();
    return this.currentProjection;
  }

  private rebuild(): void {
    this.currentProjection = this.createProjection();
    this.projectionRevision += 1;
    this.changeEmitter.fire();
  }

  private createProjection(): EditorVisualLineProjection {
    const source = this.source.ensureCurrent();
    if (!this.visibility) return source;
    const visibleLogicalLines = this.visibleLogicalLines(source.logicalLineCount);
    const lines = source.lines.filter(line => visibleLogicalLines[line.logicalLineIndex]);
    const anchors = createVisualLineAnchors(source, visibleLogicalLines, lines);
    return EditorVisualLineProjection.fromVisibleLines(source.modelVersion, source.logicalLineCount, lines, anchors);
  }

  private visibleLogicalLines(lineCount: number): readonly boolean[] {
    return Object.freeze(Array.from({ length: lineCount }, (_, lineIndex) => this.visibility!.isLineVisible(lineIndex)));
  }
}

function createVisualLineAnchors(source: EditorVisualLineProjection, visibleLogicalLines: readonly boolean[], lines: readonly { readonly logicalLineIndex: number; readonly firstForLogicalLine: boolean; readonly lastForLogicalLine: boolean }[]): readonly number[] {
  const first = Array.from({ length: source.logicalLineCount }, () => -1);
  const last = Array.from({ length: source.logicalLineCount }, () => -1);
  for (let visualLineIndex = 0; visualLineIndex < lines.length; visualLineIndex += 1) {
    const line = lines[visualLineIndex]!;
    if (line.firstForLogicalLine) first[line.logicalLineIndex] = visualLineIndex;
    if (line.lastForLogicalLine) last[line.logicalLineIndex] = visualLineIndex;
  }
  let previousVisible = -1;
  const anchors: number[] = [];
  for (let logicalLineIndex = 0; logicalLineIndex < source.logicalLineCount; logicalLineIndex += 1) {
    if (visibleLogicalLines[logicalLineIndex]) {
      previousVisible = last[logicalLineIndex]!;
      anchors.push(first[logicalLineIndex]!);
    } else {
      if (previousVisible < 0) throw new Error("A visible-line projection must retain the first logical line");
      anchors.push(previousVisible);
    }
  }
  return Object.freeze(anchors);
}
