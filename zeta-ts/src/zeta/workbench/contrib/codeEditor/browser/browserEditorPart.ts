import { CodeEditorWidget, type CodeEditorWidgetOptions } from '../../../../editor/browser/widget/codeEditor/codeEditorWidget.js';
import { createEditorBrowserServices } from '../../../../editor/browser/services/contribution.js';

export type BrowserEditorPartOptions = CodeEditorWidgetOptions;

export const editorBrowserServices = createEditorBrowserServices();

/** Creates a browser editor for a model whose language state is already model-owned. */
export function createBrowserEditorPart(options: BrowserEditorPartOptions): CodeEditorWidget {
	const editorWorkers = editorBrowserServices.workers;
	return new CodeEditorWidget({
		...options,
		codeEditorService: editorBrowserServices.codeEditorService,
		editorWorkerFactory: editorWorkers.editorWorkerFactory,
		...(options.languageFeaturesService ? {} : { completionWorkerFactory: editorWorkers.completionWorkerFactory }),
	});
}
