import assert from 'node:assert/strict';
import test from 'node:test';
import { WordNavigationType, WordOperations } from '../../common/cursor/cursorWordOperations.js';
import { SelectionStartKind, SingleCursorState } from '../../common/cursorCommon.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import { getMapForWordSeparators } from '../../common/core/wordCharacterClassifier.js';
import { ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { TextModel } from '../../common/model/textModel.js';
import { createTestCursorConfiguration } from './testCursorConfiguration.js';

test('WordOperations moves between word starts and resolves the word at a position', () => {
	using model = new TextModel('alpha beta');
	const separators = getMapForWordSeparators(' ', []);

	assert.deepEqual({
		left: WordOperations.moveWordLeft(separators, model, new Position(1, 11), WordNavigationType.WordStart, false),
		right: WordOperations.moveWordRight(separators, model, new Position(1, 1), WordNavigationType.WordStart),
		word: WordOperations.getWordAtPosition(model, ' ', [], new Position(1, 8)),
	}, {
		left: new Position(1, 7),
		right: new Position(1, 7),
		word: { word: 'beta', startColumn: 7, endColumn: 11 },
	});
});

test('WordOperations creates and extends an upstream cursor word selection', () => {
	using model = new TextModel('alpha beta');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService, { wordSeparators: ' ' });
	const cursor = new SingleCursorState(new Range(1, 8, 1, 8), SelectionStartKind.Simple, 0, new Position(1, 8), 0);

	const selected = WordOperations.word(configuration, model, cursor, false, new Position(1, 8));
	const extended = WordOperations.word(configuration, model, selected, true, new Position(1, 2));

	assert.deepEqual({ selected: selected.selection, extended: extended.selection }, {
		selected: new Selection(1, 7, 1, 11),
		extended: new Selection(1, 11, 1, 1),
	});
});

test('WordOperations computes a word deletion range without mutating the model', () => {
	using model = new TextModel('alpha beta');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const configuration = createTestCursorConfiguration(model, languageConfigurationService, { wordSeparators: ' ' });
	const range = WordOperations.deleteWordLeft({
		wordSeparators: getMapForWordSeparators(configuration.wordSeparators, configuration.wordSegmenterLocales),
		model,
		selection: Selection.fromPositions(new Position(1, 11)),
		whitespaceHeuristics: true,
		autoClosingDelete: configuration.autoClosingDelete,
		autoClosingBrackets: configuration.autoClosingBrackets,
		autoClosingQuotes: configuration.autoClosingQuotes,
		autoClosingPairs: configuration.autoClosingPairs,
		autoClosedCharacters: [],
	}, WordNavigationType.WordStart);

	assert.deepEqual({ range, text: model.getText() }, {
		range: new Range(1, 7, 1, 11),
		text: 'alpha beta',
	});
});
