import { type IMarkerDecorationsService } from '../../common/services/markerDecorations.js';
import { type IEditorContribution } from '../../common/editorCommon.js';
import { type ICodeEditor } from '../editorBrowser.js';

/** Requires marker decorations when an editor contribution is instantiated. */
export class MarkerDecorationsContribution implements IEditorContribution {
	public static readonly ID = 'editor.contrib.markerDecorations';

	constructor(_editor: ICodeEditor, _markerDecorationsService: IMarkerDecorationsService) {}

	dispose(): void {}
}
