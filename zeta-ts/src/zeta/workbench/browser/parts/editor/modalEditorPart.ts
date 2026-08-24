import "./media/modalEditorPart.css";
import { addDisposableListener, isHTMLElement, stopEvent, h } from "../../../../base/browser/dom.js";
import { focusFirst, restoreFocus, trapTabFocus } from "../../../../base/browser/focus.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";

export interface ModalEditorPartOptions {
	readonly container: HTMLElement;
	readonly title: string;
	readonly content: HTMLElement;
	readonly focusContent: () => void;
}

let nextModalEditorId = 1;

/**
 * Hosts one complex editor surface above the Workbench.
 *
 * The hosted editor owns its content and navigation. This Part owns modal
 * presentation, its compact header, close requests, and focus containment.
 */
export class ModalEditorPart extends DisposableOwner {
	readonly domNode: HTMLElement;
	readonly onDidRequestClose: Event<void>;
	private readonly host: HTMLDivElement;
	private readonly focusContent: () => void;
	private readonly _onDidRequestClose = this.own(new Emitter<void>());
	private focusToRestore: HTMLElement | undefined;
	private visible = false;

	constructor(options: ModalEditorPartOptions) {
		super();
		this.focusContent = options.focusContent;
		const ownerDocument = options.container.ownerDocument;
		this.host = h(ownerDocument, "div");
		this.host.className = "zeta-modal-editor-host";
		this.host.hidden = true;

		this.domNode = h(ownerDocument, "section");
		this.domNode.className = "zeta-modal-editor";
		this.domNode.tabIndex = -1;
		this.domNode.setAttribute("role", "dialog");
		this.domNode.setAttribute("aria-modal", "true");

		const header = h(ownerDocument, "header");
		header.className = "zeta-modal-editor-header";
		const heading = h(ownerDocument, "h2");
		heading.className = "zeta-modal-editor-title";
		heading.id = `zeta-modal-editor-title-${nextModalEditorId++}`;
		heading.textContent = options.title;
		this.domNode.setAttribute("aria-labelledby", heading.id);
		const closeButton = this.own(new Button(header, {
			label: `Close ${options.title}`,
			title: `Close ${options.title}`,
			icon: lxiconsLibrary.close,
			onClick: () => this.requestClose(),
		}));
		closeButton.toggleClassName("zeta-modal-editor-close", true);
		header.append(heading, closeButton.domNode);

		const content = h(ownerDocument, "div");
		content.className = "zeta-modal-editor-content";
		content.append(options.content);
		this.domNode.append(header, content);
		this.host.append(this.domNode);
		options.container.append(this.host);

		this.onDidRequestClose = this._onDidRequestClose.event;
		this.own(trapTabFocus(this.domNode));
		this.own(addDisposableListener(this.host, "mousedown", (event: MouseEvent) => {
			if (event.target !== this.host) return;
			stopEvent(event);
			this.requestClose();
		}));
		this.own(addDisposableListener(this.domNode, "keydown", (event: KeyboardEvent) => {
			if (event.defaultPrevented || event.isComposing || event.key !== "Escape") return;
			stopEvent(event);
			this.requestClose();
		}));
		this.defer(() => {
			this.hide();
			this.host.remove();
		});
	}

	get isVisible(): boolean {
		return this.visible;
	}

	show(): void {
		if (this.visible) {
			this.focusEditorContent();
			return;
		}
		const activeElement = this.domNode.ownerDocument.activeElement;
		this.focusToRestore = isHTMLElement(activeElement) ? activeElement : undefined;
		this.visible = true;
		this.host.hidden = false;
		this.focusEditorContent();
	}

	hide(): void {
		if (!this.visible) return;
		this.visible = false;
		this.host.hidden = true;
		const focusToRestore = this.focusToRestore;
		this.focusToRestore = undefined;
		if (focusToRestore) restoreFocus(focusToRestore);
	}

	private focusEditorContent(): void {
		this.focusContent();
		if (!this.domNode.contains(this.domNode.ownerDocument.activeElement)) {
			if (!focusFirst(this.domNode)) this.domNode.focus();
		}
	}

	private requestClose(): void {
		this._onDidRequestClose.fire();
	}
}
