import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface EditorState {
  readonly focused: boolean;
  readonly modelVersion: number;
  readonly selections: TextSelectionSet;
  readonly scrollLeft: number;
  readonly scrollTop: number;
}

/** Observable editor-instance state shared by context actions and browser contributions. */
export class EditorStateModel extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<EditorState>());
  private state: EditorState;
  readonly onDidChange: Event<EditorState> = this.changeEmitter.event;

  constructor(private readonly model: TextModel, selections: TextSelectionSet) {
    super();
    this.state = Object.freeze({ focused: false, modelVersion: model.version, selections, scrollLeft: 0, scrollTop: 0 });
    this.own(model.onDidChange(() => this.update({ modelVersion: model.version })));
  }

  get value(): EditorState {
    return this.state;
  }

  setFocused(focused: boolean): void {
    if (this.state.focused === focused) return;
    this.update({ focused });
  }

  setSelections(selections: TextSelectionSet): void {
    this.update({ selections });
  }

  setScrollPosition(scrollLeft: number, scrollTop: number): void {
    if (!Number.isFinite(scrollLeft) || !Number.isFinite(scrollTop)) throw new RangeError("Editor state scroll position must be finite");
    this.update({ scrollLeft, scrollTop });
  }

  private update(partial: Partial<EditorState>): void {
    const next = Object.freeze({ ...this.state, ...partial });
    if (next.focused === this.state.focused && next.modelVersion === this.state.modelVersion && next.selections === this.state.selections && next.scrollLeft === this.state.scrollLeft && next.scrollTop === this.state.scrollTop) return;
    this.state = next;
    this.changeEmitter.fire(next);
  }
}
