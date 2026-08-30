import assert from 'node:assert/strict';
import test from 'node:test';
import { IdentityCoordinatesConverter } from '../../common/coordinatesConverter.js';
import { CursorState } from '../../common/cursorCommon.js';
import { CursorCollection } from '../../common/cursor/cursorCollection.js';
import { CursorContext } from '../../common/cursor/cursorContext.js';
import { Cursor } from '../../common/cursor/oneCursor.js';
import { Position } from '../../common/core/position.js';
import { Range } from '../../common/core/range.js';
import { Selection } from '../../common/core/selection.js';
import { ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { TextModel } from '../../common/model/textModel.js';
import { createTestCursorConfiguration } from './testCursorConfiguration.js';

test('Cursor keeps corresponding model and view states and recovers from markers', () => {
	using model = new TextModel('abcdef');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const context = createContext(model, languageConfigurationService);
	const cursor = new Cursor(context);

	const selection = new Selection(1, 2, 1, 4);
	const state = CursorState.fromModelSelection(selection);
	cursor.setState(context, state.modelState, state.viewState);
	assert.ok(cursor.modelState.selection.equalsSelection(selection));
	assert.ok(cursor.viewState.selection.equalsSelection(selection));

	model.applyEdits([{ range: new Range(1, 1, 1, 1), text: 'X' }]);
	assert.ok(cursor.readSelectionFromMarkers(context).equalsSelection(new Selection(1, 3, 1, 5)));
	cursor.dispose(context);
});

test('CursorCollection keeps the primary cursor first and normalizes overlaps', () => {
	using model = new TextModel('abcdef\nsecond');
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	const context = createContext(model, languageConfigurationService);
	const cursors = new CursorCollection(context);

	cursors.setSelections([
		new Selection(1, 1, 1, 3),
		new Selection(1, 2, 1, 4),
	]);
	assert.equal(cursors.getAll().length, 2);
	assert.ok(cursors.getPrimaryCursor().modelState.selection.equalsSelection(new Selection(1, 1, 1, 3)));
	assert.deepEqual(cursors.getViewPositions(), [new Position(1, 3), new Position(1, 4)]);

	cursors.normalize();
	assert.equal(cursors.getAll().length, 1);
	assert.ok(cursors.getSelections()[0].equalsSelection(new Selection(1, 1, 1, 4)));
	assert.equal(cursors.getLastAddedCursorIndex(), 0);
	cursors.dispose();
});

function createContext(model: TextModel, languageConfigurationService: ComposableLanguageConfigurationService): CursorContext {
	return new CursorContext(
		model,
		model,
		new IdentityCoordinatesConverter(model),
		createTestCursorConfiguration(model, languageConfigurationService),
	);
}
