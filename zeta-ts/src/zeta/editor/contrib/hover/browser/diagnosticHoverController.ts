import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Owns Stanza's non-modal diagnostic hover over projected gutter markers. */
export class DiagnosticHoverController extends DisposableOwner {
	private readonly element: HTMLDivElement;
	private activeMarker: HTMLElement | undefined;

	constructor(private readonly viewport: EditorViewport) {
		super();
		const ownerDocument = viewport.element.ownerDocument;
		this.element = h(ownerDocument, "div");
		this.element.className = "stanza-editor-diagnostic-hover";
		this.element.hidden = true;
		this.element.setAttribute("role", "tooltip");
		(ownerDocument.body ?? viewport.element).append(this.element);
		this.defer(() => this.element.remove());
		this.own(addDisposableListener<PointerEvent>(viewport.element, "pointerover", event => this.showForTarget(event.target)));
		this.own(addDisposableListener<PointerEvent>(viewport.element, "pointerout", event => {
			if (markerForTarget(event.relatedTarget) === this.activeMarker) return;
			this.hide();
		}));
		this.own(addDisposableListener(viewport.element, "scroll", () => this.hide()));
	}

	private showForTarget(target: EventTarget | null): void {
		const marker = markerForTarget(target);
		const text = marker?.dataset.diagnosticHoverText;
		if (!marker || !text) {
			this.hide();
			return;
		}
		this.activeMarker = marker;
		this.element.textContent = text;
		this.element.hidden = false;
		const bounds = marker.getBoundingClientRect();
		const view = marker.ownerDocument.defaultView;
		const maximumLeft = Math.max(8, (view?.innerWidth ?? Number.POSITIVE_INFINITY) - 360);
		const maximumTop = Math.max(8, (view?.innerHeight ?? Number.POSITIVE_INFINITY) - 48);
		this.element.style.left = `${Math.min(Math.max(8, bounds.right + 8), maximumLeft)}px`;
		this.element.style.top = `${Math.min(Math.max(8, bounds.top), maximumTop)}px`;
	}

	private hide(): void {
		this.activeMarker = undefined;
		this.element.hidden = true;
	}
}

function markerForTarget(target: EventTarget | null): HTMLElement | undefined {
	if (!target || typeof target !== "object" || !("nodeType" in target)) return undefined;
	const node = target as Node;
	const element = node.nodeType === node.ELEMENT_NODE
		? node as HTMLElement
		: node.parentElement;
	return element?.closest<HTMLElement>(".stanza-editor-diagnostic-marker") ?? undefined;
}
