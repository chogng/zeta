import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { EditorFoldingModel } from "./foldingModel.js";
import { type EditorFoldingRegion } from "./foldingRanges.js";
import { Position } from "../../../common/core/position.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { type View } from "../../../browser/view.js";
import { type TextEditorContributionContext } from "../../../browser/editorExtensions.js";
import { SemanticMouseTargetFactory, SemanticMouseTargetKind } from "../../../browser/controller/semanticMouseTarget.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

export enum FoldingCommand {
	Collapse = "collapse",
	Expand = "expand",
	CollapseRecursively = "collapseRecursively",
	ExpandRecursively = "expandRecursively",
	CreateManualRange = "createManualRange",
	RemoveManualRange = "removeManualRange",
	CollapseToLevel = "collapseToLevel",
	CollapseAll = "collapseAll",
	ExpandAll = "expandAll",
}

export interface FoldingControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes local VS Code fold chords and gutter controls through Stanza's folding model. */
export class FoldingController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;
	private readonly viewport: View;
	private readonly selections: CursorsController;
	private readonly folding: EditorFoldingModel;
	private readonly mouseTargets: SemanticMouseTargetFactory;
	private awaitingChord = false;

	constructor(
		context: TextEditorContributionContext,
		options: FoldingControllerOptions = {},
	) {
		super();
		this.viewport = context.viewport;
		this.selections = context.selections;
		this.folding = context.getCapability(TextEditorCapability.folding);
		this.mouseTargets = new SemanticMouseTargetFactory(this.viewport);
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (this.viewport.textModel !== this.selections.textModel || this.viewport.textModel !== this.folding.model) {
				throw new TypeError("Stanza folding dependencies must share one text model");
			}
			if (context.model.largeFile.tooLargeForTokenization) return;
			this._register(addDisposableListener(context.view.element, "keydown", event => this.handleKeydown(event)));
			this._register(addDisposableListener(this.viewport.element, "pointerdown", event => this.handleGutterPointerDown(event), true));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		const chord = resolveStanzaFoldingChord(event, this.targetOperatingSystem, this.awaitingChord);
		if (chord === "prefix") {
			stopEvent(event);
			this.awaitingChord = true;
			return;
		}
		this.awaitingChord = false;
		if (chord) {
			stopEvent(event);
			if (typeof chord === "object") {
				this.setCollapsedToLevel(chord.level);
			} else if (chord === FoldingCommand.CollapseAll || chord === FoldingCommand.ExpandAll) {
				this.setAllCollapsed(chord === FoldingCommand.CollapseAll);
			} else if (chord === FoldingCommand.CreateManualRange) {
				this.createManualRange();
			} else if (chord === FoldingCommand.RemoveManualRange) {
				this.removeManualRange();
			} else {
				this.setContainingFoldRecursively(chord === FoldingCommand.CollapseRecursively);
			}
			return;
		}
		const command = resolveStanzaFoldingCommand(event, this.targetOperatingSystem);
		if (!command) return;
		stopEvent(event);
		this.setContainingFoldCollapsed(command === FoldingCommand.Collapse);
	}

	private handleGutterPointerDown(event: PointerEvent): void {
		const target = this.mouseTargets.create(event);
		if (target?.kind !== SemanticMouseTargetKind.GutterDecoration || target.decorationOwner !== "folding") return;
		const lineIndex = target.editorTarget?.position.lineNumber === undefined ? undefined : target.editorTarget.position.lineNumber - 1;
		if (lineIndex === undefined) return;
		event.preventDefault();
		event.stopPropagation();
		this.viewport.element.focus({ preventScroll: true });
		const region = this.folding.toggleAtLine(lineIndex);
		if (region?.collapsed) this.relocateHiddenSelections(region);
	}

	private setContainingFoldCollapsed(collapsed: boolean): void {
		const region = this.folding.setContainingLineCollapsed(this.selections.selections.primary.getPosition().lineNumber - 1, collapsed);
		if (!region) return;
		if (region.collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private setAllCollapsed(collapsed: boolean): void {
		if (!this.folding.setAllCollapsed(collapsed)) return;
		if (collapsed) {
			for (const region of this.folding.regions) if (region.collapsed) this.relocateHiddenSelections(region);
		}
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private setContainingFoldRecursively(collapsed: boolean): void {
		const lineIndex = this.selections.selections.primary.getPosition().lineNumber - 1;
		const region = collapsed
			? this.folding.collapseContainingRegionRecursively(lineIndex)
			: this.folding.expandContainingRegionRecursively(lineIndex);
		if (!region) return;
		if (collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private createManualRange(): void {
		const selection = this.selections.selections.primary;
		const endLineIndex = selection.endColumn === 1 && selection.endLineNumber > selection.startLineNumber
			? selection.endLineNumber - 2
			: selection.endLineNumber - 1;
		const region = this.folding.addManualRange(selection.startLineNumber - 1, endLineIndex);
		if (region) this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private removeManualRange(): void {
		const region = this.folding.removeContainingManualRange(this.selections.selections.primary.getPosition().lineNumber - 1);
		if (region) this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private setCollapsedToLevel(level: number): void {
		if (!this.folding.collapseToLevel(level)) return;
		for (const region of this.folding.regions) if (region.collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
	}

	private relocateHiddenSelections(region: EditorFoldingRegion): void {
		const header = new Position((region.startLineIndex) + 1, (this.viewport.textModel.getLineContent((region.startLineIndex) + 1).length) + 1);
		const selections = this.selections.selections.selections.map(selection => {
			const activeLineIndex = selection.getPosition().lineNumber - 1;
			return activeLineIndex > region.startLineIndex && activeLineIndex <= region.endLineIndex
				? Selection.fromPositions(header)
				: selection;
		});
		this.selections.setSelections(SelectionSet.withPrimary(selections, this.selections.selections.primaryIndex));
	}
}

function resolveStanzaFoldingChord(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem, awaitingChord: boolean): FoldingChord | undefined {
	const modifier = targetOperatingSystem === OperatingSystem.Macintosh ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
	if (!modifier || event.shiftKey || event.altKey) return undefined;
	if (!awaitingChord) return event.key.toLowerCase() === "k" ? "prefix" : undefined;
	if (event.key === "0") return FoldingCommand.CollapseAll;
	if (event.key.toLowerCase() === "j") return FoldingCommand.ExpandAll;
	if (event.key === "[") return FoldingCommand.CollapseRecursively;
	if (event.key === "]") return FoldingCommand.ExpandRecursively;
	if (event.key === ",") return FoldingCommand.CreateManualRange;
	if (event.key === ".") return FoldingCommand.RemoveManualRange;
	const level = Number(event.key);
	return Number.isSafeInteger(level) && level >= 1 && level <= 9
		? Object.freeze({ command: FoldingCommand.CollapseToLevel, level })
		: undefined;
}

type FoldingChord = FoldingCommand | "prefix" | { readonly command: FoldingCommand.CollapseToLevel; readonly level: number };

/** Resolves the platform-specific fold and unfold chords used by VS Code. */
export function resolveStanzaFoldingCommand(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): FoldingCommand | undefined {
	const command = event.key === "["
		? FoldingCommand.Collapse
		: event.key === "]"
			? FoldingCommand.Expand
			: undefined;
	if (!command) return undefined;
	if (targetOperatingSystem === OperatingSystem.Macintosh) {
		return event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey ? command : undefined;
	}
	return event.ctrlKey && event.shiftKey && !event.altKey && !event.metaKey ? command : undefined;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Stanza folding operating system");
	}
	return resolved;
}
