import { type IDisposable, DisposableOwner } from '../../../base/common/lifecycle.js';
import { type EditorViewportLayout } from '../../common/viewLayout/editorViewportModel.js';
import { type ViewportOverlayContext } from './viewportOverlay/viewportOverlayPresentation.js';

export class EditorViewContext {
	constructor(
		private readonly readLayout: () => EditorViewportLayout,
		private readonly createOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext,
	) { }

	public get layout(): EditorViewportLayout {
		return this.readLayout();
	}

	public overlayContext(layout: EditorViewportLayout): ViewportOverlayContext | undefined {
		const context = this.createOverlayContext(layout);
		return context.visualLineProjection.modelVersion === context.model.version ? context : undefined;
	}
}

export interface EditorViewPart extends IDisposable {
	render(layout: EditorViewportLayout): void;
}

export abstract class EditorOverlayPart extends DisposableOwner implements EditorViewPart {
	protected constructor(protected readonly context: EditorViewContext) {
		super();
	}

	public abstract render(layout: EditorViewportLayout): void;
}

export class EditorViewPartCollection extends DisposableOwner {
	private readonly parts: EditorViewPart[] = [];

	public register<TPart extends EditorViewPart>(part: TPart): TPart {
		this.parts.push(part);
		this.own(part);
		return part;
	}

	public render(layout: EditorViewportLayout): void {
		for (const part of this.parts) {
			part.render(layout);
		}
	}
}
