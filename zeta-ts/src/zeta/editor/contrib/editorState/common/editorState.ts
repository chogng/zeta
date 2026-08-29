import { Emitter, type Event } from "../../../../base/common/event.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import type { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface EditorState {
	readonly focused: boolean;
	readonly modelVersion: number;
	readonly selections: SelectionSet;
	readonly scrollLeft: number;
	readonly scrollTop: number;
}

/** Observable editor-instance state shared by context actions and browser contributions. */
export class EditorStateModel extends Disposable {
	private readonly changeEmitter = this._register(new Emitter<EditorState>());
	private state: EditorState;
	readonly onDidChange: Event<EditorState> = this.changeEmitter.event;

	constructor(private readonly model: TextModel, selections: SelectionSet) {
		super();
		this.state = Object.freeze({ focused: false, modelVersion: model.version, selections, scrollLeft: 0, scrollTop: 0 });
		this._register(model.onDidChange(() => this.update({ modelVersion: model.version })));
	}

	get value(): EditorState {
		return this.state;
	}

	setFocused(focused: boolean): void {
		if (this.state.focused === focused) return;
		this.update({ focused });
	}

	setSelections(selections: SelectionSet): void {
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
