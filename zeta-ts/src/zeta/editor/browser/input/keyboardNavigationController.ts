import { addDisposableListener } from "../../../base/browser/dom.js";
import { StandardKeyboardEvent } from "../../../base/browser/keyboardEvent.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../base/common/platform.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, navigateEditorCursors } from "../../common/cursor/cursorNavigation.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../view/editorViewport.js";
import { EditorLineWrapping } from "../viewModel/visualLineProjection.js";
import { navigateAsterVisualCursors } from "../../common/viewModel/visualCursorNavigation.js";

export interface KeyboardNavigationControllerOptions {
	readonly operatingSystem?: OperatingSystem;
	/** Resolves the active language word matcher for word navigation. */
	readonly wordPattern?: () => RegExp | undefined;
}

export interface KeyboardNavigationCommand {
	readonly command: EditorCursorNavigationCommand;
	readonly mode: EditorCursorNavigationMode;
}

/**
 * Routes browser keydown navigation into Aster common selection commands.
 */
export class KeyboardNavigationController extends DisposableOwner {
	private readonly targetOperatingSystem: OperatingSystem;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private preferredColumns: readonly number[] | undefined;
	private preferredVisualHorizontalOffsets: readonly number[] | undefined;
	private applyingNavigation = false;

	constructor(
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: KeyboardNavigationControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(
				options.operatingSystem,
			);
			if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
				throw new TypeError("Aster keyboard word pattern resolver must be a function");
			}
			this.wordPattern = options.wordPattern;
		} catch (error) {
			this.dispose();
			throw error;
		}
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Aster keyboard and selection controllers must share one text model",
			);
		}
		this.own(addDisposableListener(
			viewport.element,
			"keydown",
			event => this.handleKeydown(event),
		));
		this.own(selectionController.onDidChange(() => {
			if (!this.applyingNavigation) {
				this.preferredColumns = undefined;
				this.preferredVisualHorizontalOffsets = undefined;
			}
		}));
	}

	private handleKeydown(browserEvent: KeyboardEvent): void {
		if (browserEvent.defaultPrevented) return;
		const event = new StandardKeyboardEvent(browserEvent);
		const navigation = resolveAsterKeyboardNavigation(
			event,
			this.targetOperatingSystem,
		);
		if (!navigation) return;
		event.stop();
		const layout = this.viewport.viewportLayout;
		const pageLineCount = Math.max(
			1,
			Math.floor(layout.viewportSize.height / layout.lineHeight),
		);
		const visualCommand = isVisualVerticalCommand(navigation.command)
			? navigation.command
			: undefined;
		const result = this.viewport.lineWrapping === EditorLineWrapping.On &&
			visualCommand !== undefined
			? navigateAsterVisualCursors(
				this.viewport.textModel,
				this.viewport.getVisualLineProjection(),
				this.selectionController.selections,
				{
					command: visualCommand,
					mode: navigation.mode,
					pageLineCount,
					preferredHorizontalOffsets: this.preferredVisualHorizontalOffsets,
				},
				text => this.viewport.measureTextWidth(text),
				{
					getHorizontalOffset: position => this.viewport.getVisualHorizontalOffset(position),
					getNearestPosition: (visualLineIndex, horizontalOffset) => this.viewport.getNearestPositionAtVisualHorizontalOffset(visualLineIndex, horizontalOffset),
				},
			)
			: navigateEditorCursors(
				this.viewport.textModel,
				this.selectionController.selections,
				{
					...navigation,
					pageLineCount,
					...(this.wordPattern ? { wordPattern: this.wordPattern() } : {}),
					preferredColumns: this.preferredColumns,
				},
			);
		this.applyingNavigation = true;
		try {
			this.selectionController.setSelections(result.selections);
		} finally {
			this.applyingNavigation = false;
		}
		if ("preferredHorizontalOffsets" in result) {
			this.preferredColumns = undefined;
			this.preferredVisualHorizontalOffsets = result.preferredHorizontalOffsets;
		} else {
			this.preferredColumns = result.preferredColumns;
			this.preferredVisualHorizontalOffsets = undefined;
		}
		this.viewport.revealPosition(result.selections.primary.active);
	}
}

function isVisualVerticalCommand(command: EditorCursorNavigationCommand): command is EditorCursorNavigationCommand.LineUp | EditorCursorNavigationCommand.LineDown | EditorCursorNavigationCommand.PageUp | EditorCursorNavigationCommand.PageDown {
	return command === EditorCursorNavigationCommand.LineUp ||
		command === EditorCursorNavigationCommand.LineDown ||
		command === EditorCursorNavigationCommand.PageUp ||
		command === EditorCursorNavigationCommand.PageDown;
}

export function resolveAsterKeyboardNavigation(event: Pick<StandardKeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey" | "altGraphKey" | "isComposing">, targetOperatingSystem: OperatingSystem): KeyboardNavigationCommand | undefined {
	if (event.isComposing || event.altGraphKey) return undefined;
	const mode = event.shiftKey
		? EditorCursorNavigationMode.Extend
		: EditorCursorNavigationMode.Move;
	const noCommandModifier =
		!event.ctrlKey && !event.altKey && !event.metaKey;
	if (noCommandModifier) {
		const command = unmodifiedCommand(event.key);
		return command ? { command, mode } : undefined;
	}

	if (targetOperatingSystem === OperatingSystem.Macintosh) {
		if (event.altKey && !event.ctrlKey && !event.metaKey) {
			if (event.key === "ArrowLeft") {
				return { command: EditorCursorNavigationCommand.WordLeft, mode };
			}
			if (event.key === "ArrowRight") {
				return { command: EditorCursorNavigationCommand.WordRight, mode };
			}
		}
		if (event.metaKey && !event.ctrlKey && !event.altKey) {
			const command = macCommandCommand(event.key);
			return command ? { command, mode } : undefined;
		}
		return undefined;
	}

	if (event.ctrlKey && !event.altKey && !event.metaKey) {
		const command = controlCommand(event.key);
		return command ? { command, mode } : undefined;
	}
	return undefined;
}

function unmodifiedCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.CharacterLeft;
		case "ArrowRight":
			return EditorCursorNavigationCommand.CharacterRight;
		case "ArrowUp":
			return EditorCursorNavigationCommand.LineUp;
		case "ArrowDown":
			return EditorCursorNavigationCommand.LineDown;
		case "Home":
			return EditorCursorNavigationCommand.LineStart;
		case "End":
			return EditorCursorNavigationCommand.LineEnd;
		case "PageUp":
			return EditorCursorNavigationCommand.PageUp;
		case "PageDown":
			return EditorCursorNavigationCommand.PageDown;
		default:
			return undefined;
	}
}

function controlCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.WordLeft;
		case "ArrowRight":
			return EditorCursorNavigationCommand.WordRight;
		case "Home":
			return EditorCursorNavigationCommand.DocumentStart;
		case "End":
			return EditorCursorNavigationCommand.DocumentEnd;
		default:
			return undefined;
	}
}

function macCommandCommand(key: string): EditorCursorNavigationCommand | undefined {
	switch (key) {
		case "ArrowLeft":
			return EditorCursorNavigationCommand.LineStart;
		case "ArrowRight":
			return EditorCursorNavigationCommand.LineEnd;
		case "ArrowUp":
		case "Home":
			return EditorCursorNavigationCommand.DocumentStart;
		case "ArrowDown":
		case "End":
			return EditorCursorNavigationCommand.DocumentEnd;
		default:
			return undefined;
	}
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Aster keyboard operating system");
	}
	return resolved;
}
