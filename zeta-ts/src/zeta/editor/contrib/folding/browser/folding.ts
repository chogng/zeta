import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { EditorFoldingModel } from "./foldingModel.js";
import { type EditorFoldingRegion } from "./foldingRanges.js";
import { TextPosition } from "../../../common/core/text.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type EditorViewport } from "../../../browser/view.js";

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
export class FoldingController extends DisposableOwner {
	private readonly targetOperatingSystem: OperatingSystem;
	private awaitingChord = false;

	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		private readonly folding: EditorFoldingModel,
		options: FoldingControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== selections.textModel || viewport.textModel !== folding.model) {
				throw new TypeError("Stanza folding dependencies must share one text model");
			}
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
			this.own(addDisposableListener(viewport.element, "pointerdown", event => this.handleGutterPointerDown(event)));
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
		const target = event.target as { closest?: <T extends Element>(selector: string) => T | null } | null;
		const button = target?.closest?.<HTMLButtonElement>(".stanza-editor-fold-toggle");
		if (!button || !this.viewport.element.contains(button)) return;
		const lineIndex = Number(button.dataset.logicalLineIndex);
		if (!Number.isSafeInteger(lineIndex)) return;
		event.preventDefault();
		event.stopPropagation();
		this.viewport.element.focus({ preventScroll: true });
		const region = this.folding.toggleAtLine(lineIndex);
		if (region?.collapsed) this.relocateHiddenSelections(region);
	}

	private setContainingFoldCollapsed(collapsed: boolean): void {
		const region = this.folding.setContainingLineCollapsed(this.selections.selections.primary.active.lineIndex, collapsed);
		if (!region) return;
		if (region.collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private setAllCollapsed(collapsed: boolean): void {
		if (!this.folding.setAllCollapsed(collapsed)) return;
		if (collapsed) {
			for (const region of this.folding.regions) if (region.collapsed) this.relocateHiddenSelections(region);
		}
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private setContainingFoldRecursively(collapsed: boolean): void {
		const lineIndex = this.selections.selections.primary.active.lineIndex;
		const region = collapsed
			? this.folding.collapseContainingRegionRecursively(lineIndex)
			: this.folding.expandContainingRegionRecursively(lineIndex);
		if (!region) return;
		if (collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private createManualRange(): void {
		const selection = this.selections.selections.primary.range;
		const endLineIndex = selection.end.columnIndex === 0 && selection.end.lineIndex > selection.start.lineIndex
			? selection.end.lineIndex - 1
			: selection.end.lineIndex;
		const region = this.folding.addManualRange(selection.start.lineIndex, endLineIndex);
		if (region) this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private removeManualRange(): void {
		const region = this.folding.removeContainingManualRange(this.selections.selections.primary.active.lineIndex);
		if (region) this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private setCollapsedToLevel(level: number): void {
		if (!this.folding.collapseToLevel(level)) return;
		for (const region of this.folding.regions) if (region.collapsed) this.relocateHiddenSelections(region);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private relocateHiddenSelections(region: EditorFoldingRegion): void {
		const header = TextPosition.at(
			region.startLineIndex,
			this.viewport.textModel.getLineContent(region.startLineIndex).length,
		);
		const selections = this.selections.selections.selections.map(selection => {
			const activeLineIndex = selection.active.lineIndex;
			return activeLineIndex > region.startLineIndex && activeLineIndex <= region.endLineIndex
				? TextSelection.collapsedAt(header)
				: selection;
		});
		this.selections.setSelections(TextSelectionSet.withPrimary(selections, this.selections.selections.primaryIndex));
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
