import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { type Event } from "../../../../base/common/event.js";
import { type Icon } from "../../../../base/common/icon.js";
import { Disposable, type IDisposable } from "../../../../base/common/lifecycle.js";

export const EDITOR_GUTTER_SLOT_WIDTH = 20;

/** Semantic state rendered by the editor's shared line-gutter renderer. */
export interface EditorLineGutterItem {
	readonly className: string;
	readonly icon?: Icon;
	readonly label: string;
	readonly title?: string;
	readonly expanded?: boolean;
	readonly pressed?: boolean;
}

/** Supplies feature state without owning browser DOM or icon rendering. */
export interface EditorLineGutterDecoration extends IDisposable {
	readonly onDidChange: Event<void>;
	getDecoration(logicalLineIndex: number, firstForLogicalLine: boolean): EditorLineGutterItem | undefined;
	activate?(logicalLineIndex: number): void;
}

/** Owns stable gutter DOM, semantic icon rendering, accessibility, and activation routing. */
export class EditorLineGutterRenderer extends Disposable {
	readonly width: number;

	constructor(
		private readonly host: HTMLElement,
		readonly decorations: readonly EditorLineGutterDecoration[],
		private readonly onDidChange: () => void,
	) {
		super();
		if (decorations.length === 0) throw new RangeError("A gutter renderer requires at least one decoration");
		this.width = decorations.length * EDITOR_GUTTER_SLOT_WIDTH;
		for (const decoration of decorations) {
			this._register(decoration);
			this._register(decoration.onDidChange(this.onDidChange));
		}
		this._register(addDisposableListener(this.host, "click", event => this.activate(event)));
	}

	create(ownerDocument: Document): HTMLElement {
		const root = h(ownerDocument, "span");
		root.className = "stanza-editor-feature-gutter";
		this.decorations.forEach((_decoration, index) => {
			const slot = h(ownerDocument, "span");
			slot.className = "stanza-editor-feature-gutter-slot";
			const button = h(ownerDocument, "button");
			button.className = "stanza-editor-gutter-decoration";
			button.type = "button";
			button.hidden = true;
			button.dataset.gutterDecorationIndex = String(index);
			slot.append(button);
			root.append(slot);
		});
		return root;
	}

	render(element: HTMLElement, logicalLineIndex: number, firstForLogicalLine: boolean): void {
		if (element.children.length !== this.decorations.length) throw new Error("Editor gutter decoration DOM is out of sync");
		this.decorations.forEach((decoration, index) => {
			const button = element.children[index]?.firstElementChild;
			if (!(button instanceof element.ownerDocument.defaultView!.HTMLButtonElement)) throw new TypeError("Editor gutter decoration slot is invalid");
			this.renderButton(button, decoration.getDecoration(logicalLineIndex, firstForLogicalLine), logicalLineIndex);
		});
	}

	private renderButton(button: HTMLButtonElement, item: EditorLineGutterItem | undefined, logicalLineIndex: number): void {
		button.hidden = !item;
		if (!item) {
			button.className = "stanza-editor-gutter-decoration";
			delete button.dataset.logicalLineIndex;
			delete button.dataset.iconId;
			button.replaceChildren();
			button.removeAttribute("aria-label");
			button.removeAttribute("aria-expanded");
			button.removeAttribute("aria-pressed");
			button.removeAttribute("title");
			return;
		}

		button.className = `stanza-editor-gutter-decoration ${item.className}`;
		button.dataset.logicalLineIndex = String(logicalLineIndex);
		button.setAttribute("aria-label", item.label);
		setOptionalBooleanAttribute(button, "aria-expanded", item.expanded);
		setOptionalBooleanAttribute(button, "aria-pressed", item.pressed);
		if (item.title === undefined) button.removeAttribute("title");
		else button.title = item.title;

		const iconId = item.icon?.id;
		if (button.dataset.iconId === iconId) return;
		button.replaceChildren();
		if (item.icon) {
			appendIcon(item.icon, button);
			button.dataset.iconId = item.icon.id;
		} else {
			delete button.dataset.iconId;
		}
	}

	private activate(event: globalThis.Event): void {
		const target = event.target;
		if (!(target instanceof this.host.ownerDocument.defaultView!.Element)) return;
		const button = target.closest<HTMLButtonElement>(".stanza-editor-gutter-decoration");
		if (!button || !this.host.contains(button)) return;
		const decorationIndex = Number(button.dataset.gutterDecorationIndex);
		const logicalLineIndex = Number(button.dataset.logicalLineIndex);
		if (!Number.isSafeInteger(decorationIndex) || !Number.isSafeInteger(logicalLineIndex) || logicalLineIndex < 0) return;
		this.decorations[decorationIndex]?.activate?.(logicalLineIndex);
	}
}

function setOptionalBooleanAttribute(element: HTMLElement, name: string, value: boolean | undefined): void {
	if (value === undefined) element.removeAttribute(name);
	else element.setAttribute(name, String(value));
}
