import { type EditorRenderingContext } from './renderingContext.js';
import { EditorViewPart } from './viewPart.js';

/** Base for overlays rendered from one editor view context. */
export abstract class DynamicViewOverlay extends EditorViewPart {
	override prepareRender(_context: EditorRenderingContext): void {}

	abstract override render(context: EditorRenderingContext): void;
}
