import { type EditorRenderingContext } from './renderingContext.js';
import { EditorViewPart, type EditorViewContext } from './viewPart.js';

/** Base for browser overlays whose DOM is projected from the current view. */
export abstract class EditorDynamicViewOverlay extends EditorViewPart {
	protected constructor(protected readonly context: EditorViewContext) {
		super();
	}

	public abstract override render(context: EditorRenderingContext): void;
}
