import { AbstractCodeEditorService } from './abstractCodeEditorService.js';
import { registerEditorContribution } from '../editorExtensions.js';
import { EditorWorkerService } from './editorWorkerService.js';
import { MarkerDecorationsContribution } from './markerDecorations.js';

export function registerEditorBrowserContributions(): void {
	registerEditorContribution(MarkerDecorationsContribution);
}

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
