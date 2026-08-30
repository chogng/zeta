import { AbstractCodeEditorService } from './abstractCodeEditorService.js';
import { registerTextEditorCapabilityContribution } from '../editorExtensions.js';
import { EditorBrowserWorkerFactories } from './editorBrowserWorkerFactories.js';
import { MarkerDecorationsContribution } from './markerDecorations.js';

export function registerEditorBrowserContributions(): void {
	registerTextEditorCapabilityContribution(MarkerDecorationsContribution);
}

class BrowserCodeEditorService extends AbstractCodeEditorService {}

export interface EditorBrowserServices {
	readonly codeEditors: BrowserCodeEditorService;
	readonly workers: EditorBrowserWorkerFactories;
}

/** Creates the browser-owned editor services used by a composition root. */
export function createEditorBrowserServices(): EditorBrowserServices {
	return Object.freeze({
		codeEditors: new BrowserCodeEditorService(),
		workers: new EditorBrowserWorkerFactories(),
	});
}
