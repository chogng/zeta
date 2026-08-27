import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { clamp } from "../../../../base/common/numbers.js";
import { type EditorScrollPosition } from "../../../common/viewModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/viewLayout.js";

/**
 * Owns pointer navigation for one rendered Stanza minimap.
 *
 * This controller does not own the viewport or its model. It maps a primary
 * pointer's vertical position to canonical scroll state and continues that
 * mapping while the pointer is dragged outside the narrow minimap element.
 */
export class MinimapNavigationController extends Disposable {
	private pointerId: number | undefined;

	constructor(
		private readonly element: HTMLElement,
		private readonly readLayout: () => EditorViewportLayout,
		private readonly scrollTo: (position: EditorScrollPosition) => void,
	) {
		super();
		const ownerDocument = element.ownerDocument;
		this._register(addDisposableListener<PointerEvent>(element, "pointerdown", event => this.begin(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, "pointermove", event => this.move(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, "pointerup", event => this.end(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, "pointercancel", event => this.end(event)));
		this._register(toDisposable(() => this.element.classList.remove("dragging")));
	}

	private begin(event: PointerEvent): void {
		if (event.button !== 0) return;
		const layout = this.readLayout();
		if (layout.viewportSize.height <= 0) return;
		this.pointerId = readPointerId(event);
		this.element.classList.add("dragging");
		event.preventDefault();
		this.scrollAt(event.clientY, layout);
	}

	private move(event: PointerEvent): void {
		if (this.pointerId === undefined || readPointerId(event) !== this.pointerId) return;
		event.preventDefault();
		this.scrollAt(event.clientY, this.readLayout());
	}

	private end(event: PointerEvent): void {
		if (this.pointerId === undefined || readPointerId(event) !== this.pointerId) return;
		this.pointerId = undefined;
		this.element.classList.remove("dragging");
	}

	private scrollAt(clientY: number, layout: EditorViewportLayout): void {
		if (!Number.isFinite(clientY) || layout.viewportSize.height <= 0) return;
		const bounds = this.element.getBoundingClientRect();
		const fraction = clamp((clientY - bounds.top) / layout.viewportSize.height, 0, 1);
		this.scrollTo({ left: layout.scrollPosition.left, top: fraction * layout.maximumScrollPosition.top });
	}
}

function readPointerId(event: PointerEvent): number {
	return Number.isSafeInteger(event.pointerId) ? event.pointerId : 0;
}
