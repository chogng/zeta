import { addDisposableListener, getWindow } from '../../../../base/browser/dom.js';
import { scheduleAtNextAnimationFrame } from '../../../../base/browser/scheduler.js';
import { Disposable, MutableDisposable, type IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import type { ICodeEditor } from '../../../browser/editorBrowser.js';
import type { TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import type { IEditorContribution } from '../../../common/editorCommon.js';
import './middleScroll.css';

interface MiddleScrollSession {
	readonly pointerId: number;
	readonly startX: number;
	readonly startY: number;
	readonly dotDomNode: HTMLDivElement;
	x: number;
	y: number;
	lastFrame: number;
	didScroll: boolean;
}

/** Owns one editor's middle-click continuous scrolling session. */
export class MiddleScrollController extends Disposable implements IEditorContribution {
	public static readonly ID = 'editor.contrib.middleScroll';

	public static get(editor: ICodeEditor): MiddleScrollController | null {
		return editor.getContribution<MiddleScrollController>(MiddleScrollController.ID);
	}

	private readonly frame = this._register(new MutableDisposable<IDisposable>());
	private session: MiddleScrollSession | undefined;

	constructor(private readonly context: TextEditorContributionContext) {
		super();
		if (!context.options.scrollOnMiddleClick) return;
		const targetWindow = getWindow(context.viewport.element);
		this._register(addDisposableListener<PointerEvent>(context.viewport.element, 'pointerdown', event => this.handlePointerDown(event)));
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointermove', event => this.handlePointerMove(event)));
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointerup', event => this.handlePointerUp(event)));
		this._register(addDisposableListener<PointerEvent>(targetWindow, 'pointercancel', event => this.handlePointerCancel(event)));
		this._register(addDisposableListener(context.view.element, 'keydown', () => this.stop()));
		this._register(toDisposable(() => this.stop()));
	}

	private handlePointerDown(event: PointerEvent): void {
		if (event.button !== 1) return;
		event.preventDefault();
		event.stopPropagation();
		if (this.session) {
			this.stop();
			return;
		}
		const bounds = this.context.viewport.element.getBoundingClientRect();
		const dotDomNode = this.context.viewport.element.ownerDocument.createElement('div');
		dotDomNode.className = 'scroll-editor-on-middle-click-dot';
		dotDomNode.style.left = `${event.clientX - bounds.left}px`;
		dotDomNode.style.top = `${event.clientY - bounds.top}px`;
		this.context.viewport.element.append(dotDomNode);
		this.context.viewport.element.classList.add('scroll-editor-on-middle-click-editor');
		this.session = {
			pointerId: event.pointerId,
			startX: event.clientX,
			startY: event.clientY,
			x: event.clientX,
			y: event.clientY,
			lastFrame: performance.now(),
			didScroll: false,
			dotDomNode,
		};
		this.scheduleFrame();
	}

	private handlePointerMove(event: PointerEvent): void {
		if (!this.session || event.pointerId !== this.session.pointerId) return;
		this.session.x = event.clientX;
		this.session.y = event.clientY;
		this.context.viewport.element.dataset.scrollDirection = direction(this.session.x - this.session.startX, this.session.y - this.session.startY);
	}

	private handlePointerUp(event: PointerEvent): void {
		if (this.session?.pointerId === event.pointerId && this.session.didScroll) this.stop();
	}

	private handlePointerCancel(event: PointerEvent): void {
		if (this.session?.pointerId === event.pointerId) this.stop();
	}

	private scheduleFrame(): void {
		const targetWindow = getWindow(this.context.viewport.element);
		this.frame.value = scheduleAtNextAnimationFrame(targetWindow, () => {
			this.frame.clear();
			const session = this.session;
			if (!session) return;
			const now = performance.now();
			const factor = Math.min(2, Math.max(0, now - session.lastFrame) / 32);
			session.lastFrame = now;
			const x = afterThreshold(session.x - session.startX);
			const y = afterThreshold(session.y - session.startY);
			if (x !== 0 || y !== 0) {
				const before = this.context.viewport.currentLayout.scrollPosition;
				const after = this.context.viewport.scrollTo({ left: before.left + x * factor, top: before.top + y * factor }).scrollPosition;
				session.didScroll ||= before.left !== after.left || before.top !== after.top;
			}
			this.scheduleFrame();
		});
	}

	private stop(): void {
		this.frame.clear();
		this.session?.dotDomNode.remove();
		this.session = undefined;
		this.context.viewport.element.classList.remove('scroll-editor-on-middle-click-editor');
		delete this.context.viewport.element.dataset.scrollDirection;
	}
}

function afterThreshold(delta: number): number {
	if (Math.abs(delta) <= 5) return 0;
	return delta - Math.sign(delta) * 5;
}

function direction(x: number, y: number): string {
	const vertical = y < -5 ? 'n' : y > 5 ? 's' : '';
	const horizontal = x < -5 ? 'w' : x > 5 ? 'e' : '';
	return vertical + horizontal;
}
