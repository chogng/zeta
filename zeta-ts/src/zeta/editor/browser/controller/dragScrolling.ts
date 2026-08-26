import { AnimationFrameScheduler } from "../../../base/browser/scheduler.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { clamp } from "../../../base/common/numbers.js";
import { type EditorViewport } from "../view/editorViewport.js";
import { type ClientPoint, type EditorHitTarget } from "../../common/viewModel/pointerHitTest.js";

const MINIMUM_SPEED = 240;
const MAXIMUM_SPEED = 2_400;
const SPEED_PER_OVERFLOW_PIXEL = 18;
const DEFAULT_FRAME_DURATION = 1_000 / 60;
const MINIMUM_FRAME_DURATION = 4;
const MAXIMUM_FRAME_DURATION = 50;

export interface DragScrollVelocity {
	readonly left: number;
	readonly top: number;
}

export interface DragScrollBounds {
	readonly left: number;
	readonly top: number;
	readonly right: number;
	readonly bottom: number;
}

/**
 * Maps pointer overflow beyond a viewport to pixels per second.
 *
 * Each axis remains independent, starts at a usable minimum speed, and is
 * capped so a distant captured pointer cannot jump through a document.
 */
export function getStanzaDragScrollVelocity(bounds: DragScrollBounds, point: ClientPoint): DragScrollVelocity {
	validateBounds(bounds);
	validatePoint(point);
	return Object.freeze({
		left: axisVelocity(point.clientX, bounds.left, bounds.right),
		top: axisVelocity(point.clientY, bounds.top, bounds.bottom),
	});
}

/**
 * Owns animation-frame scrolling for one active pointer drag.
 */
/** Owns animation-frame scrolling for one active editor drag. */
export class DragScrolling extends DisposableOwner {
	private readonly scheduler: AnimationFrameScheduler;
	private pointer: ClientPoint | undefined;
	private lastFrameTime: number | undefined;

	constructor(
		private readonly targetWindow: Window,
		private readonly viewport: EditorViewport,
		private readonly applyTarget: (target: EditorHitTarget) => void,
	) {
		super();
		this.scheduler = this.own(new AnimationFrameScheduler(
			targetWindow,
			() => this.runFrame(),
		));
	}

	updatePointer(point: ClientPoint): void {
		validatePoint(point);
		this.pointer = Object.freeze({
			clientX: point.clientX,
			clientY: point.clientY,
		});
		const velocity = this.readVelocity(this.pointer);
		if (velocity.left === 0 && velocity.top === 0) {
			this.scheduler.cancel();
			this.lastFrameTime = undefined;
		} else {
			this.scheduler.schedule();
		}
	}

	private runFrame(): void {
		const pointer = this.pointer;
		if (!pointer) return;
		const velocity = this.readVelocity(pointer);
		if (velocity.left === 0 && velocity.top === 0) {
			this.lastFrameTime = undefined;
			return;
		}
		const now = this.targetWindow.performance.now();
		const elapsed = this.lastFrameTime === undefined
			? DEFAULT_FRAME_DURATION
			: clamp(now - this.lastFrameTime, MINIMUM_FRAME_DURATION, MAXIMUM_FRAME_DURATION);
		this.lastFrameTime = now;
		const before = this.viewport.viewportLayout.scrollPosition;
		const layout = this.viewport.scrollTo({
			left: before.left + velocity.left * elapsed / 1_000,
			top: before.top + velocity.top * elapsed / 1_000,
		});
		const target = this.viewport.getNearestTargetAtClientPoint(pointer);
		if (target) this.applyTarget(target);
		if (
			layout.scrollPosition.left !== before.left ||
			layout.scrollPosition.top !== before.top
		) {
			this.scheduler.schedule();
		} else {
			this.lastFrameTime = undefined;
		}
	}

	private readVelocity(point: ClientPoint): DragScrollVelocity {
		return getStanzaDragScrollVelocity(
			this.viewport.element.getBoundingClientRect(),
			point,
		);
	}
}

function axisVelocity(position: number, minimum: number, maximum: number): number {
	if (position < minimum) return -speedForOverflow(minimum - position);
	if (position >= maximum) return speedForOverflow(position - maximum);
	return 0;
}

function speedForOverflow(overflow: number): number {
	return Math.min(
		MAXIMUM_SPEED,
		MINIMUM_SPEED + overflow * SPEED_PER_OVERFLOW_PIXEL,
	);
}

function validatePoint(point: ClientPoint): void {
	if (
		!point ||
		!Number.isFinite(point.clientX) ||
		!Number.isFinite(point.clientY)
	) {
		throw new RangeError("Stanza autoscroll point must contain finite coordinates");
	}
}

function validateBounds(bounds: DragScrollBounds): void {
	if (
		!bounds ||
		!Number.isFinite(bounds.left) ||
		!Number.isFinite(bounds.top) ||
		!Number.isFinite(bounds.right) ||
		bounds.right < bounds.left ||
		!Number.isFinite(bounds.bottom) ||
		bounds.bottom < bounds.top
	) {
		throw new RangeError("Stanza autoscroll bounds are invalid");
	}
}
