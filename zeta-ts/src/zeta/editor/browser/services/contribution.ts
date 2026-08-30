import { AbstractWidgetCodeEditorRegistry } from './abstractCodeEditorService.js';
import { registerTextEditorCapabilityContribution } from '../editorExtensions.js';
import { EditorBrowserWorkerFactories } from './editorBrowserWorkerFactories.js';
import { MarkerDecorationsContribution } from './markerDecorations.js';

export function registerEditorBrowserContributions(): void {
	registerTextEditorCapabilityContribution(MarkerDecorationsContribution);
}

class BrowserWidgetCodeEditorRegistry extends AbstractWidgetCodeEditorRegistry {}

export interface EditorBrowserServices {
	readonly codeEditors: BrowserWidgetCodeEditorRegistry;
	readonly workers: EditorBrowserWorkerFactories;
}

/** Creates the browser-owned editor services used by a composition root. */
export function createEditorBrowserServices(): EditorBrowserServices {
	return Object.freeze({
		codeEditors: new BrowserWidgetCodeEditorRegistry(),
		workers: new EditorBrowserWorkerFactories(),
	});
}
