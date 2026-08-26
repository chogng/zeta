import { DisposableOwner } from '../../../base/common/lifecycle.js';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { type EditorRenderingContext } from './renderingContext.js';

export type { EditorRenderingContext } from './renderingContext.js';

export class EditorViewContext {
	constructor(
		private readonly readLayout: () => EditorViewportLayout,
		private readonly createRenderingContext: (layout: EditorViewportLayout) => EditorRenderingContext,
	) { }

	public get layout(): EditorViewportLayout {
		return this.readLayout();
	}

	public get renderingContext(): EditorRenderingContext {
		return this.createRenderingContext(this.layout);
	}
}

export abstract class EditorViewPart extends DisposableOwner {
	public prepareRender(_context: EditorRenderingContext): void {
	}

	public renderNow(context: EditorRenderingContext): void {
		this.prepareRender(context);
		this.render(context);
	}

	public abstract render(context: EditorRenderingContext): void;
}

export class EditorViewPartCollection extends DisposableOwner {
	private readonly parts: EditorViewPart[] = [];

	public register<TPart extends EditorViewPart>(part: TPart): TPart {
		this.parts.push(part);
		this.own(part);
		return part;
	}

	public prepareRender(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.prepareRender(context);
		}
	}

	public render(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.render(context);
		}
	}
}
