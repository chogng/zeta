import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { Disposable, toDisposable } from "../../../../../base/common/lifecycle.js";
import { type Event } from "../../../../../base/common/event.js";
import { IME } from "../../../../../base/common/ime.js";
import { type IAccessibilityService } from "../../../../../platform/accessibility/common/accessibility.js";
import { type CursorsController } from "../../../../common/cursor/cursor.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { type TextModel } from "../../../../common/model/textModel.js";
import { type EditorViewport } from "../../../view.js";
import { RichScreenReaderContent } from "./screenReaderContentRich.js";
import { SimpleScreenReaderContent } from "./screenReaderContentSimple.js";
import { createScreenReaderContentState, DEFAULT_SCREEN_READER_PAGE_SIZE, screenReaderLineOffsetAtModelOffset, type NativeScreenReaderContent } from "./screenReaderUtils.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "../../../viewparts/viewLines/viewLine.js";

export interface NativeScreenReaderSupportOptions {
	readonly element: HTMLElement;
	readonly model: TextModel;
	readonly viewport: EditorViewport;
	readonly selectionController: CursorsController;
	/** Logical focus events from NativeEditContext; they hide the IME bridge hop. */
	readonly onDidFocus?: Event<void>;
	readonly onDidBlur?: Event<void>;
	readonly accessibilityService?: IAccessibilityService;
	readonly renderRichContent?: boolean;
	readonly accessibilityPageSize?: number;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	readonly isComposing?: () => boolean;
}

/**
 * Owns the screen-reader-only projection required by native EditContext.
 *
 * The native browser buffer remains the input authority. This controller only
 * exposes a bounded, paged source-text mirror when accessibility optimization
 * is on and keeps that mirror aligned with the viewport's active cursor.
 */
export class ScreenReaderSupport extends Disposable {
	private readonly content: NativeScreenReaderContent;
	private focused = false;
	private syncScheduled = false;
	private previousSelectionChangeEventTime = 0;

	constructor(private readonly options: NativeScreenReaderSupportOptions) {
		super();
		this._register(toDisposable(() => this.resetNativeScreenReaderLayout()));
		this.content = this._register(options.renderRichContent
			? new RichScreenReaderContent(options.element, {
				model: options.model,
				semanticTokenSource: options.semanticTokenSource,
				bracketColorizationSource: options.bracketColorizationSource,
			})
			: new SimpleScreenReaderContent(options.element));
		if (options.onDidFocus) {
			this._register(options.onDidFocus(() => this.handleFocusChange(true)));
		} else {
			this._register(addDisposableListener(options.element, "focus", () => this.handleFocusChange(true)));
		}
		if (options.onDidBlur) {
			this._register(options.onDidBlur(() => this.handleFocusChange(false)));
		} else {
			this._register(addDisposableListener(options.element, "blur", () => this.handleFocusChange(false)));
		}
		this._register(addDisposableListener(options.element, "cut", () => this.onWillCut()));
		this._register(addDisposableListener(options.element, "paste", () => this.onWillPaste()));
		this._register(addDisposableListener(options.element.ownerDocument, "selectionchange", () => this.acceptDomSelection()));
		this._register(options.model.onDidChange(() => this.scheduleSynchronization()));
		this._register(options.selectionController.onDidChange(() => this.scheduleSynchronization()));
		this._register(options.viewport.onDidChangeLayout(() => this.layoutContent()));
		if (options.semanticTokenSource) {
			this._register(options.semanticTokenSource.onDidChange(() => this.scheduleSynchronization()));
		}
		if (options.accessibilityService) {
			this._register(options.accessibilityService.onDidChangeScreenReaderOptimized(() => this.scheduleSynchronization()));
		}
		this.scheduleSynchronization();
	}

	get isEnabled(): boolean {
		return this.options.accessibilityService?.isScreenReaderOptimized() ?? false;
	}

	onWillCut(): void {
		this.content.setIgnoreSelectionChange();
	}

	onWillPaste(): void {
		this.content.setIgnoreSelectionChange();
	}

	writeScreenReaderContent(): void {
		this.synchronize();
	}

	private handleFocusChange(focused: boolean): void {
		if (this.focused === focused) return;
		this.focused = focused;
		if (focused) this.scheduleSynchronization();
		else {
			this.content.clear();
			this.resetNativeScreenReaderLayout();
		}
	}

	private scheduleSynchronization(): void {
		if (this.syncScheduled) return;
		this.syncScheduled = true;
		queueMicrotask(() => {
			this.syncScheduled = false;
			this.synchronize();
		});
	}

	private synchronize(): void {
		if (
			this.isDisposed ||
			!this.isEnabled ||
			!this.focused ||
			this.options.isComposing?.()
		) {
			this.content.clear();
			this.resetNativeScreenReaderLayout();
			return;
		}
		this.content.sync(createScreenReaderContentState(
			this.options.model,
			this.options.selectionController.selections.primary,
			{ pageSize: this.options.accessibilityPageSize ?? DEFAULT_SCREEN_READER_PAGE_SIZE },
		));
		this.layoutContent();
	}

	private layoutContent(): void {
		const state = this.content.getState();
		if (
			!state ||
			this.isDisposed ||
			!this.isEnabled ||
			!this.focused ||
			this.options.isComposing?.()
		) {
			this.resetNativeScreenReaderLayout();
			return;
		}

		const viewportLayout = this.options.viewport.currentLayout;
		const selection = this.options.selectionController.selections.primary;
		const position = this.options.viewport.getPositionContentCoordinates(selection.active);
		const scrollPosition = viewportLayout.scrollPosition;
		const viewportWidth = viewportLayout.viewportSize.width;
		const viewportHeight = viewportLayout.viewportSize.height;
		const cursorVisible = viewportWidth > 0 && viewportHeight > 0 &&
			position.left >= scrollPosition.left &&
			position.left <= scrollPosition.left + viewportWidth &&
			position.top >= scrollPosition.top &&
			position.top <= scrollPosition.top + viewportHeight;
		const textLeft = this.options.viewport.getPositionContentCoordinates(TextPosition.at(0, 0)).left;
		const desiredLeft = cursorVisible ? textLeft - scrollPosition.left : 0;
		const desiredTop = cursorVisible ? position.top - scrollPosition.top : 0;
		const lineHeight = Math.max(1, position.height);
		const rootLeft = readInlinePixel(this.options.element.style.left);
		const rootTop = readInlinePixel(this.options.element.style.top);
		this.options.element.classList.add("stanza-native-screen-reader-content-active");
		this.content.layout({
			left: desiredLeft - rootLeft,
			top: desiredTop - rootTop,
			width: Math.max(1, viewportWidth),
			height: lineHeight,
			lineHeight,
			scrollTop: screenReaderLineOffsetAtModelOffset(state, this.options.model.offsetAt(selection.active)) * lineHeight,
		});
	}

	private resetNativeScreenReaderLayout(): void {
		this.options.element.classList.remove("stanza-native-screen-reader-content-active");
	}

	private acceptDomSelection(): void {
		if (
			this.isDisposed ||
			!this.isEnabled ||
			!this.focused ||
			this.options.isComposing?.() ||
			this.content.shouldIgnoreSelectionChange()
		) return;
		if (!IME.enabled) return;
		const now = Date.now();
		if (now - this.previousSelectionChangeEventTime < 5) return;
		this.previousSelectionChangeEventTime = now;
		const domSelection = this.content.readSelection();
		if (!domSelection) return;
		const model = this.options.model;
		const anchor = model.positionAt(domSelection.anchorOffset);
		const active = model.positionAt(domSelection.activeOffset);
		const current = this.options.selectionController.selections.primary;
		if (current.anchor.equals(anchor) && current.active.equals(active)) return;
		this.options.selectionController.setSelections(TextSelectionSet.single(TextSelection.from(anchor, active)));
		this.options.viewport.revealPosition(active);
	}
}

function readInlinePixel(value: string): number {
	const parsed = Number.parseFloat(value);
	return Number.isFinite(parsed) ? parsed : 0;
}
