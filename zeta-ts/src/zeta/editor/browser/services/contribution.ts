import { AbstractCodeEditorService } from './abstractCodeEditorService.js';
import { EditorWorkerService } from './editorWorkerService.js';

class BrowserCodeEditorService extends AbstractCodeEditorService {}

export interface EditorBrowserServices {
	readonly codeEditors: BrowserCodeEditorService;
	readonly workers: EditorWorkerService;
}

/** Creates the browser-owned editor services used by a composition root. */
export function createEditorBrowserServices(): EditorBrowserServices {
	return Object.freeze({
		codeEditors: new BrowserCodeEditorService(),
		workers: new EditorWorkerService(),
	});
}
