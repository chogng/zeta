import assert from 'node:assert/strict';
import test from 'node:test';
import { DisposableStore } from '../../../base/common/lifecycle.js';
import { URI } from '../../../base/common/uri.js';
import { AbstractWidgetCodeEditorRegistry } from '../../browser/services/abstractCodeEditorService.js';
import { type CodeEditorWidget } from '../../browser/widget/codeEditor/codeEditorWidget.js';

class TestWidgetCodeEditorRegistry extends AbstractWidgetCodeEditorRegistry {}

test('code editor handlers remove their own ordered registration', async () => {
	using service = new TestWidgetCodeEditorRegistry();
	const sharedEditor = { id: 'shared' } as unknown as CodeEditorWidget;
	const middleEditor = { id: 'middle' } as unknown as CodeEditorWidget;
	const shared = async (): Promise<CodeEditorWidget> => sharedEditor;
	const middle = async (): Promise<CodeEditorWidget> => middleEditor;
	using registrations = new DisposableStore();
	const oldShared = registrations.add(service.registerCodeEditorOpenHandler(shared));
	registrations.add(service.registerCodeEditorOpenHandler(middle));
	registrations.add(service.registerCodeEditorOpenHandler(shared));

	oldShared.dispose();

	assert.equal(await service.openCodeEditor(URI.parse('file:///document.ts')), sharedEditor);
});
