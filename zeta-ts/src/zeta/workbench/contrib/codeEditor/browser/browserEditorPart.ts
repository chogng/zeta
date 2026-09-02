import { CodeEditorWidget, type CodeEditorWidgetOptions } from '../../../../editor/browser/widget/codeEditor/codeEditorWidget.js';
import { createEditorBrowserServices } from '../../../../editor/browser/services/contribution.js';
import { ServiceContainer } from '../../../../platform/instantiation/common/instantiation.js';
import { ILogService, NullLoggerService } from '../../../../platform/log/common/log.js';

export type BrowserEditorPartOptions = CodeEditorWidgetOptions;

export const editorBrowserServices = createEditorBrowserServices();
const editorBrowserServiceContainer = new ServiceContainer();
editorBrowserServiceContainer.registerInstance(ILogService, new NullLoggerService());

/** Creates a browser editor for a model whose language state is already model-owned. */
export function createBrowserEditorPart(options: BrowserEditorPartOptions): CodeEditorWidget {
	const editorWorkers = editorBrowserServices.workers;
	return new CodeEditorWidget({
		...options,
		instantiationService: options.instantiationService ?? editorBrowserServiceContainer,
		codeEditorService: editorBrowserServices.codeEditorService,
		editorWorkerFactory: editorWorkers.editorWorkerFactory,
		...(options.languageFeaturesService ? {} : { completionWorkerFactory: editorWorkers.completionWorkerFactory }),
	});
}
