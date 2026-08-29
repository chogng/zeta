import { RunOnceScheduler } from "../../../../base/common/async.js";
import { isFiniteNumber } from "../../../../base/common/numbers.js";
import { HorizontalScrollbar } from "../../../../base/browser/ui/scrollbar/horizontalScrollbar.js";
import { VerticalScrollbar } from "../../../../base/browser/ui/scrollbar/verticalScrollbar.js";
import { createScrollbarAxisMetrics, type ScrollbarAxisMetrics } from "../../../../base/browser/ui/scrollbar/scrollbarState.js";
import { EditorOptions } from "../../../common/config/editorOptions.js";
import { type EditorScrollPosition } from "../../../common/viewModel.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

export type EditorScrollbarVisibility = "auto" | "visible" | "hidden";

export interface EditorScrollbarOptions {
	readonly container: HTMLElement;
	readonly viewport: HTMLElement;
	readonly scrollTo: (position: EditorScrollPosition) => void;
	readonly horizontalScrollbarSize?: number;
	readonly verticalScrollbarSize?: number;
	readonly minimumThumbSize?: number;
	readonly horizontal?: EditorScrollbarVisibility;
	readonly vertical?: EditorScrollbarVisibility;
}

/**
 * Projects the editor viewport's canonical scroll state into two themed
 * scrollbar axes. Native scrolling remains the editor's input and
 * accessibility fallback; this part owns only the visible custom tracks.
 */
export class EditorScrollbar extends EditorViewPart {
	private static nextViewportId = 1;
	private readonly container: HTMLElement;
	private readonly horizontal: HorizontalScrollbar;
	private readonly vertical: VerticalScrollbar;
	private readonly horizontalScrollbarSize: number;
	private readonly verticalScrollbarSize: number;
	private readonly minimumThumbSize: number;
	private readonly horizontalVisibility: EditorScrollbarVisibility;
	private readonly verticalVisibility: EditorScrollbarVisibility;
	private horizontalMetrics: ScrollbarAxisMetrics;
	private verticalMetrics: ScrollbarAxisMetrics;
	private lastScrollPosition: EditorScrollPosition | undefined;
	private readonly scrollActivityScheduler: RunOnceScheduler;

	constructor(options: EditorScrollbarOptions) {
		super();
		if (!options.container || !options.viewport) {
			throw new TypeError("Editor scrollbar part requires a container and viewport");
		}
		if (typeof options.scrollTo !== "function") {
			throw new TypeError("Editor scrollbar part requires a scroll callback");
		}
		this.container = options.container;
		this.horizontalScrollbarSize = positiveFinite(options.horizontalScrollbarSize ?? EditorOptions.scrollbar.defaultValue.horizontalScrollbarSize, "horizontalScrollbarSize");
		this.verticalScrollbarSize = positiveFinite(options.verticalScrollbarSize ?? EditorOptions.scrollbar.defaultValue.verticalScrollbarSize, "verticalScrollbarSize");
		this.minimumThumbSize = positiveFinite(options.minimumThumbSize ?? 20, "minimumThumbSize");
		this.horizontalVisibility = options.horizontal ?? "auto";
		this.verticalVisibility = options.vertical ?? "auto";
		if (!options.viewport.id) {
			options.viewport.id = `stanza-editor-scroll-viewport-${EditorScrollbar.nextViewportId++}`;
		}
		options.container.style.setProperty("--stanza-editor-horizontal-scrollbar-size", `${this.horizontalScrollbarSize}px`);
		options.container.style.setProperty("--stanza-editor-vertical-scrollbar-size", `${this.verticalScrollbarSize}px`);
		this.horizontalMetrics = createScrollbarAxisMetrics(0, 0, 0, 0, 0);
		this.verticalMetrics = createScrollbarAxisMetrics(0, 0, 0, 0, 0);
		this.horizontal = this._register(new HorizontalScrollbar(options.container, {
			viewport: options.viewport,
			trackClickBehavior: "jump",
			getMetrics: () => this.horizontalMetrics,
			setPosition: position => options.scrollTo({
				left: position,
				top: this.verticalMetrics.position,
			}),
		}));
		this.vertical = this._register(new VerticalScrollbar(options.container, {
			viewport: options.viewport,
			trackClickBehavior: "jump",
			getMetrics: () => this.verticalMetrics,
			setPosition: position => options.scrollTo({
				left: this.horizontalMetrics.position,
				top: position,
			}),
		}));
		this.configureTrack(this.horizontal, "horizontal", this.horizontalVisibility);
		this.configureTrack(this.vertical, "vertical", this.verticalVisibility);
		this.scrollActivityScheduler = this._register(new RunOnceScheduler(() => {
			this.container.classList.remove("stanza-editor-scrolling");
		}, 700));
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		if (
			this.lastScrollPosition !== undefined &&
			(this.lastScrollPosition.left !== layout.scrollPosition.left ||
				this.lastScrollPosition.top !== layout.scrollPosition.top)
		) {
			this.showScrollbars();
		}
		this.lastScrollPosition = layout.scrollPosition;
		const horizontalRendered = isRendered(
			this.horizontalVisibility,
			layout.maximumScrollPosition.left > 0,
		);
		const verticalRendered = isRendered(
			this.verticalVisibility,
			layout.maximumScrollPosition.top > 0,
		);
		const horizontalTrackSize = Math.max(
			0,
			layout.viewportSize.width - (verticalRendered ? this.verticalScrollbarSize : 0),
		);
		const verticalTrackSize = Math.max(
			0,
			layout.viewportSize.height - (horizontalRendered ? this.horizontalScrollbarSize : 0),
		);
		this.horizontal.trackNode.setRight(verticalRendered ? this.verticalScrollbarSize : 0);
		this.vertical.trackNode.setBottom(horizontalRendered ? this.horizontalScrollbarSize : 0);
		const scrollTransform = `translate3d(${layout.scrollPosition.left}px, ${layout.scrollPosition.top}px, 0)`;
		this.horizontal.trackNode.setTransform(scrollTransform);
		this.vertical.trackNode.setTransform(scrollTransform);
		this.horizontalMetrics = createScrollbarAxisMetrics(
			layout.viewportSize.width,
			layout.contentSize.width,
			layout.scrollPosition.left,
			horizontalTrackSize,
			this.minimumThumbSize,
		);
		this.verticalMetrics = createScrollbarAxisMetrics(
			layout.viewportSize.height,
			layout.contentSize.height,
			layout.scrollPosition.top,
			verticalTrackSize,
			this.minimumThumbSize,
		);
		this.horizontal.render(this.horizontalMetrics, horizontalRendered);
		this.vertical.render(this.verticalMetrics, verticalRendered);
	}

	private configureTrack(
		scrollbar: HorizontalScrollbar | VerticalScrollbar,
		axis: "horizontal" | "vertical",
		visibility: EditorScrollbarVisibility,
	): void {
		scrollbar.trackNode.toggleClassName("stanza-editor-scrollbar-track", true);
		scrollbar.trackNode.toggleClassName(`stanza-editor-scrollbar-track-${axis}`, true);
		scrollbar.track.dataset.visibility = visibility;
	}

	private showScrollbars(): void {
		this.container.classList.add("stanza-editor-scrolling");
		this.scrollActivityScheduler.schedule();
	}
}

function isRendered(visibility: EditorScrollbarVisibility, needed: boolean): boolean {
	return visibility === "visible" || (visibility === "auto" && needed);
}

function positiveFinite(value: number, name: string): number {
	if (!isFiniteNumber(value) || value <= 0) throw new RangeError(`${name} must be positive and finite`);
	return value;
}
