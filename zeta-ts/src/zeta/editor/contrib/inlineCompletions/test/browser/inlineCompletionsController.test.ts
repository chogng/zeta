import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Emitter } from '../../../../../base/common/event.js';
import { h } from '../../../../../base/browser/dom.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { TriggerInlineEditCommandsRegistry } from '../../../../browser/triggerInlineEditCommandsRegistry.js';
import { OwnedLanguageFeatureProviderRegistry } from '../../../../common/ownedLanguageFeatureProviderRegistry.js';
import { CursorsController } from '../../../../common/cursor/cursor.js';
import { Selection } from '../../../../common/core/selection.js';
import { SelectionSet } from '../../../../common/cursor/selectionSet.js';
import { Position } from '../../../../common/core/position.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type LanguageInlineCompletionsProvider } from '../../common/inlineCompletions.js';
import { InlineCompletionsService } from '../../../../browser/services/inlineCompletionsService.js';

class TestResizeObserver {
	observe(): void {}
	unobserve(): void {}
	disconnect(): void {}
}

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	ResizeObserver: TestResizeObserver,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import('../../../../browser/view.js');
const { InlineCompletionProviderService } = await import('../../../../browser/services/inlineCompletionProviderService.js');
const { EditorInlineCompletionsController } = await import('../../browser/controller/inlineCompletionsController.js');

test.after(() => browserEnvironment.window.close());

test('Registered editor commands retrigger inline completions after their edit', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector<HTMLElement>('main')!;
	using model = new TextModel('abc');
	using selections = new CursorsController(model, SelectionSet.single(Selection.fromPositions(new Position((0) + 1, (3) + 1))));
	using viewport = new View({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
	viewport.layout({ width: 200, height: 40 });
	const input = h(dom.window.document, 'textarea');
	container.append(input);
	using providers = new OwnedLanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>();
	const requests: string[] = [];
	providers.register({
		languageIds: ['plaintext'],
		provideInlineCompletions: request => {
			requests.push(request.triggerKind);
			return [{ insertText: ' completion' }];
		},
	});
	using service = new InlineCompletionProviderService(model, providers);
	using inlineCompletionsService = new InlineCompletionsService();
	using commands = new Emitter<{ readonly commandId: string }>();
	const commandId = 'editor.test.inlineCompletionTrigger';
	TriggerInlineEditCommandsRegistry.registerCommand(commandId);
	using controller = new EditorInlineCompletionsController(input, viewport, selections, service, inlineCompletionsService, 'plaintext', commands.event);

	commands.fire({ commandId: 'editor.test.unrelatedCommand' });
	await flushPromises();
	assert.deepEqual(requests, []);
	commands.fire({ commandId });
	await flushPromises();
	assert.deepEqual(requests, ['automatic']);
	assert.equal(viewport.element.querySelector('.stanza-editor-inline-completion')?.textContent, ' completion');

	dom.window.close();
});

test('InlineCompletionsService owns snooze state and change events', () => {
	using service = new InlineCompletionsService();
	const changes: boolean[] = [];
	using listener = service.onDidChangeIsSnoozing(value => changes.push(value));
	service.snooze(10_000);
	assert.equal(service.isSnoozing(), true);
	assert.ok(service.snoozeTimeLeft > 0);
	service.snooze(10_000);
	assert.deepEqual(changes, [true]);
	service.cancelSnooze();
	assert.equal(service.isSnoozing(), false);
	assert.deepEqual(changes, [true, false]);
	assert.throws(() => service.setSnoozeDuration(-1), /non-negative/);
	assert.throws(() => service.reportNewCompletion(''), /non-empty string/);
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
