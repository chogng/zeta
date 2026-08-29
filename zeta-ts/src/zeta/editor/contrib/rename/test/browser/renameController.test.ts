import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { h } from '../../../../../base/browser/dom.js';
import { URI } from '../../../../../base/common/uri.js';
import { type EditorCommandExecutor } from '../../../../browser/editorExtensions.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { LanguageFeatureProviderRegistry } from '../../../../common/languageFeatureRegistry.js';
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { TextSelection, TextSelectionSet } from '../../../../common/core/selection.js';
import { TextPosition, TextRange } from '../../../../common/core/text.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type LanguageRenameProvider, RenameService } from '../../common/rename.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorViewport } = await import('../../../../browser/view.js');
const { RenameCommandId, RenameController } = await import('../../browser/renameController.js');

test.after(() => browserEnvironment.window.close());

test('Rename reports its command after applying the provider edit', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	const resource = URI.file('C:\\project\\rename.ts');
	using model = new TextModel('abc');
	using selections = new CursorsController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 1))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 40 });
	const editorInput = h(dom.window.document, 'textarea');
	container.append(editorInput);
	using providers = new LanguageFeatureProviderRegistry<LanguageRenameProvider>();
	providers.register({
		languageIds: ['typescript'],
		prepareRename: () => ({ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)), placeholder: 'abc' }),
		provideRenameEdits: () => ({ entries: [{ kind: 'textDocument', resource, edits: [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)), text: 'xyz' }] }] }),
	});
	using service = new RenameService(model, resource, providers);
	const executedCommands: string[] = [];
	const executeCommand: EditorCommandExecutor = (commandId, operation) => {
		executedCommands.push(commandId);
		return operation();
	};
	const errors: unknown[] = [];
	using controller = new RenameController(editorInput, viewport, selections, service, 'typescript', resource, undefined, error => errors.push(error), executeCommand);

	editorInput.dispatchEvent(keydown(dom.window, 'F2'));
	await flushPromises();
	const renameInput = viewport.element.querySelector<HTMLInputElement>('.stanza-editor-rename-input');
	assert.ok(renameInput);
	assert.equal(renameInput.value, 'abc');
	renameInput.value = 'xyz';
	renameInput.dispatchEvent(keydown(dom.window, 'Enter'));
	await flushPromises();
	assert.equal(model.getText(), 'xyz');
	assert.deepEqual(executedCommands, [RenameCommandId]);
	assert.deepEqual(errors, []);

	dom.window.close();
});

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return text.length * 10;
	}
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string): KeyboardEvent {
	return new targetWindow.KeyboardEvent('keydown', { bubbles: true, cancelable: true, key }) as unknown as KeyboardEvent;
}

async function flushPromises(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
	await Promise.resolve();
}
