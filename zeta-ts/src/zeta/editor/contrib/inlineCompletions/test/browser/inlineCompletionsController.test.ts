import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Emitter } from '../../../../../base/common/event.js';
import { h } from '../../../../../base/browser/dom.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { TriggerInlineEditCommandsRegistry } from '../../../../browser/triggerInlineEditCommandsRegistry.js';
import { LanguageFeatureProviderRegistry } from '../../../../common/languageFeatureRegistry.js';
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { TextSelection, TextSelectionSet } from '../../../../common/core/selection.js';
import { TextPosition } from '../../../../common/core/text.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type LanguageInlineCompletionsProvider } from '../../common/inlineCompletions.js';

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
const { InlineCompletionsService } = await import('../../../../browser/services/inlineCompletionsService.js');
const { InlineCompletionsController } = await import('../../browser/inlineCompletionsController.js');

test.after(() => browserEnvironment.window.close());

test('Registered editor commands retrigger inline completions after their edit', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	using model = new TextModel('abc');
	using selections = new CursorsController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 3))));
	using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 40 });
	const input = h(dom.window.document, 'textarea');
	container.append(input);
	using providers = new LanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>();
	const requests: string[] = [];
	providers.register({
		languageIds: ['plaintext'],
		provideInlineCompletions: request => {
			requests.push(request.triggerKind);
			return [{ insertText: ' completion' }];
		},
	});
	using service = new InlineCompletionsService(model, providers);
	using commands = new Emitter<{ readonly commandId: string }>();
	const commandId = 'editor.test.inlineCompletionTrigger';
	TriggerInlineEditCommandsRegistry.registerCommand(commandId);
	using controller = new InlineCompletionsController(input, viewport, selections, service, 'plaintext', commands.event);

	commands.fire({ commandId: 'editor.test.unrelatedCommand' });
	await flushPromises();
	assert.deepEqual(requests, []);
	commands.fire({ commandId });
	await flushPromises();
	assert.deepEqual(requests, ['automatic']);
	assert.equal(viewport.element.querySelector('.stanza-editor-inline-completion')?.textContent, ' completion');

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

async function flushPromises(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
}
