import "./media/folding.css";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { register } from "../../../../base/common/icon.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type EditorLineGutterDecoration, type EditorLineGutterItem } from "../../../browser/viewparts/margin/lineGutterDecoration.js";
import { type EditorFoldingModel } from "./foldingModel.js";

export const foldingExpandedIcon = register("folding-expanded", lxiconsLibrary.chevronDown);
export const foldingCollapsedIcon = register("folding-collapsed", lxiconsLibrary.chevronRight);

/** Owns folding gutter presentation and mirrors every fold-state change. */
export class FoldingDecorationProvider extends Disposable implements EditorLineGutterDecoration {
	private readonly changeEmitter = this._register(new Emitter<void>());

	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(private readonly folding: EditorFoldingModel) {
		super();
		this._register(folding.onDidChange(() => this.changeEmitter.fire()));
	}

	getDecoration(logicalLineIndex: number, firstForLogicalLine: boolean): EditorLineGutterItem | undefined {
		const region = firstForLogicalLine
			? this.folding.regions.find(candidate => candidate.startLineIndex === logicalLineIndex)
			: undefined;
		if (!region) return undefined;
		return {
			className: "stanza-editor-fold-toggle",
			icon: region.collapsed ? foldingCollapsedIcon : foldingExpandedIcon,
			label: region.collapsed ? "Expand folded lines" : "Collapse lines",
			expanded: !region.collapsed,
		};
	}
}
