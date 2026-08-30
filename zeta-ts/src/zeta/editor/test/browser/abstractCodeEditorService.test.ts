import assert from 'node:assert/strict';
import test from 'node:test';
import { DisposableStore } from '../../../base/common/lifecycle.js';
import { URI } from '../../../base/common/uri.js';
import { AbstractCodeEditorService } from '../../browser/services/abstractCodeEditorService.js';
import { type ICodeEditor } from '../../browser/editorBrowser.js';

class TestCodeEditorService extends AbstractCodeEditorService { }

test('code editor open handlers use newest-first registration and dispose independently', async () => {
	using service = new TestCodeEditorService();
	const first = { getId: () => 'first' } as unknown as ICodeEditor;
	const second = { getId: () => 'second' } as unknown as ICodeEditor;
	using registrations = new DisposableStore();
	const old = registrations.add(service.registerCodeEditorOpenHandler(async () => first));
	registrations.add(service.registerCodeEditorOpenHandler(async () => second));
	assert.equal(await service.openCodeEditor({ resource: URI.parse('file:///document.ts') }, null), second);
	old.dispose();
	assert.equal(await service.openCodeEditor({ resource: URI.parse('file:///document.ts') }, null), second);
});
