import { type EditorRenderingContext } from './renderingContext.js';
import { EditorViewPart, type EditorViewContext } from './viewPart.js';

/** Base for overlays rendered from one editor view context. */
export abstract class DynamicViewOverlay extends EditorViewPart {
	protected constructor(protected readonly context: EditorViewContext) {
		super();
	}

	abstract override render(context: EditorRenderingContext): void;
}
