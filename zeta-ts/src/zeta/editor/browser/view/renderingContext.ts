import { type EditorViewportLayout } from '../../common/viewLayout/editorViewportModel.js';
import { type ViewportOverlayContext } from '../viewparts/viewportOverlay/viewportOverlayPresentation.js';

/**
 * Immutable state shared by every Part during one synchronous render pass.
 *
 * The overlay snapshot is omitted when the visual projection is not at the
 * same model version as the layout. Non-overlay Parts can still project the
 * layout while overlay consumers skip stale geometry as one group.
 */
export interface EditorRenderingContext {
	readonly layout: EditorViewportLayout;
	readonly overlay: ViewportOverlayContext | undefined;
}

/** Creates the version-bound context used by one render pass. */
export function createEditorRenderingContext(layout: EditorViewportLayout, overlay: ViewportOverlayContext): EditorRenderingContext {
	return Object.freeze({
		layout,
		overlay: overlay.visualLineProjection.modelVersion === overlay.model.version ? overlay : undefined,
	});
}
