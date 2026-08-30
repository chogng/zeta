import assert from 'node:assert/strict';
import test from 'node:test';
import { DeleteOperations } from '../../common/cursor/cursorDeleteOperations.js';
import { EditOperationType } from '../../common/cursorCommon.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import type { ICommand, IEditOperationBuilder } from '../../common/editorCommon.js';
import { ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { TextModel } from '../../common/model/textModel.js';
import { createTestCursorConfiguration } from './testCursorConfiguration.js';

test('DeleteOperations deletes one grapheme to the left', () => {
	using model = new TextModel('a😀b');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService);

	const [pushStackElement, commands] = DeleteOperations.deleteLeft(
		EditOperationType.Other,
		configuration,
		model,
		[Selection.fromPositions(new Position(1, 4))],
		[],
	);

	assert.deepEqual({ pushStackElement, edits: readEdits(commands) }, {
		pushStackElement: true,
		edits: [{ range: new Range(1, 2, 1, 4), text: '' }],
	});
});

test('DeleteOperations uses indentation tab stops for backspace', () => {
	using model = new TextModel('    value');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService, { useTabStops: true });

	const [, commands] = DeleteOperations.deleteLeft(
		EditOperationType.DeletingLeft,
		configuration,
		model,
		[Selection.fromPositions(new Position(1, 5))],
		[],
	);

	assert.deepEqual(readEdits(commands), [{ range: new Range(1, 1, 1, 5), text: '' }]);
});

test('DeleteOperations trims leading whitespace when joining lines to the right', () => {
	using model = new TextModel('a\n  b');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService, { trimWhitespaceOnDelete: true });

	const [pushStackElement, commands] = DeleteOperations.deleteRight(
		EditOperationType.DeletingRight,
		configuration,
		model,
		[Selection.fromPositions(new Position(1, 2))],
	);

	assert.deepEqual({ pushStackElement, edits: readEdits(commands) }, {
		pushStackElement: true,
		edits: [{ range: new Range(1, 2, 2, 3), text: '' }],
	});
});

test('DeleteOperations removes only editor-tracked auto-closing pairs in auto mode', () => {
	using model = new TextModel('()');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	using registration = languageConfigurationService.register('plaintext', { autoClosingPairs: [{ open: '(', close: ')' }] });
	const configuration = createTestCursorConfiguration(model, languageConfigurationService, { autoClosingDelete: 'auto' });
	const selection = Selection.fromPositions(new Position(1, 2));

	const [, untracked] = DeleteOperations.deleteLeft(EditOperationType.Other, configuration, model, [selection], []);
	const [, tracked] = DeleteOperations.deleteLeft(EditOperationType.Other, configuration, model, [selection], [new Range(1, 2, 1, 3)]);

	assert.deepEqual({ untracked: readEdits(untracked), tracked: readEdits(tracked) }, {
		untracked: [{ range: new Range(1, 1, 1, 2), text: '' }],
		tracked: [{ range: new Range(1, 1, 1, 3), text: '' }],
	});
});

function readEdits(commands: readonly (ICommand | null)[]): readonly { readonly range: Range; readonly text: string | null }[] {
	return commands.flatMap(command => {
		if (!command) return [];
		const edits: Array<{ range: Range; text: string | null }> = [];
		const builder: IEditOperationBuilder = {
			addEditOperation: (range, text) => edits.push({ range: Range.lift(range), text }),
			addTrackedEditOperation: (range, text) => edits.push({ range: Range.lift(range), text }),
			trackSelection: () => 'unused',
		};
		command.getEditOperations({} as TextModel, builder);
		return edits;
	});
}
