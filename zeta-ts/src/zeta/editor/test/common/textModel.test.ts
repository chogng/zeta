import assert from "node:assert/strict";
import test from "node:test";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { TextModelChangeReason } from "../../common/core/textChange.js";
import { EndOfLinePreference, EndOfLineSequence, PositionAffinity, TrackedRangeStickiness, isITextSnapshot } from '../../common/model.js';
import { TextModel } from "../../common/model/textModel.js";

const position = (lineIndex: number, columnIndex: number): Position => new Position(lineIndex + 1, columnIndex + 1);
const range = (
	startLine: number,
	startColumn: number,
	endLine: number,
	endColumn: number,
): Range => Range.fromPositions(
	position(startLine, startColumn),
	position(endLine, endColumn),
);

test("TextModel normalizes line endings and maps UTF-16 positions", () => {
	using model = new TextModel("A😀\r\nbeta\rgamma\u2028");

	assert.deepEqual({
		text: model.getText(),
		lineCount: model.lineCount,
		lines: [0, 1, 2].map(index => model.getLineContent((index) + 1)),
		emojiEndOffset: model.offsetAt(position(0, 3)),
		emojiInterior: model.positionAt(2),
		end: model.positionAt(model.getText().length),
	}, {
		text: "A😀\r\nbeta\r\ngamma\u2028",
		lineCount: 3,
		lines: ["A😀", "beta", "gamma\u2028"],
		emojiEndOffset: 3,
		emojiInterior: position(0, 2),
		end: position(2, 6),
	});
	assert.equal(model.mightContainUnusualLineTerminators(), true);
});

test("TextModel exposes allocation-free document and line lengths", () => {
	using model = new TextModel("alpha\n😀");

	assert.equal(model.length, 8);
	assert.equal(model.getLineLength((0) + 1), 5);
	assert.equal(model.getLineLength((1) + 1), 2);
	assert.throws(() => model.getLineLength((2) + 1), /lineNumber/);
});

test('TextModel exposes the editor model reading contract', () => {
	using model = new TextModel('\uFEFF\talpha\n  \nאב😀');

	assert.deepEqual(model.getLinesContent(), ['\talpha', '  ', 'אב😀']);
	assert.equal(model.getValue(), '\talpha\n  \nאב😀');
	assert.equal(model.getValue(EndOfLinePreference.CRLF), '\talpha\r\n  \r\nאב😀');
	assert.equal(model.getValue(EndOfLinePreference.TextDefined, true), '\uFEFF\talpha\n  \nאב😀');
	assert.equal(model.getValueLength(EndOfLinePreference.CRLF, true), 17);
	assert.equal(model.getCharacterCountInRange(model.getFullModelRange()), 13);
	assert.equal(model.getEOL(), '\n');
	assert.equal(model.getEndOfLineSequence(), EndOfLineSequence.LF);
	assert.equal(model.getLineMinColumn(1), 1);
	assert.equal(model.getLineFirstNonWhitespaceColumn(1), 2);
	assert.equal(model.getLineFirstNonWhitespaceColumn(2), 0);
	assert.equal(model.getLineLastNonWhitespaceColumn(1), 7);
	assert.equal(model.getLineLastNonWhitespaceColumn(2), 0);
	assert.equal(model.getLineIndentColumn(1), 2);
	assert.equal(model.getLineIndentColumn(2), 3);
	assert.equal(model.mightContainRTL(), true);
	assert.equal(model.mightContainUnusualLineTerminators(), false);
	assert.equal(model.mightContainNonBasicASCII(), true);
	assert.equal(model.getLanguageIdAtPosition(1, 1), 'plaintext');
	assert.deepEqual(model.normalizePosition(new Position(1, 2), PositionAffinity.Left), new Position(1, 2));
	assert.equal(model.isValidRange(new Range(1, 1, 3, 5)), true);
	assert.equal(model.isValidRange({ startLineNumber: 3, startColumn: 5, endLineNumber: 1, endColumn: 1 }), false);
	assert.deepEqual(model.modifyPosition(new Position(1, 1), 2), new Position(1, 3));
	assert.deepEqual(model.modifyPosition(new Position(1, 1), -1), new Position(1, 1));
	assert.deepEqual(model.modifyPosition(new Position(3, 3), 1), new Position(3, 4));
	assert.deepEqual(model.modifyPosition(new Position(3, 5), 100), new Position(3, 5));
});

test('TextModel owns word lookup at a model position', () => {
	using model = new TextModel('alpha  beta.\ntail');

	assert.deepEqual(model.getWordAtPosition(position(0, 2)), {
		word: 'alpha',
		startColumn: 1,
		endColumn: 6,
	});
	assert.deepEqual(model.getWordAtPosition(position(0, 5)), {
		word: 'alpha',
		startColumn: 1,
		endColumn: 6,
	});
	assert.equal(model.getWordAtPosition(position(0, 6)), null);
	assert.deepEqual(model.getWordAtPosition(position(0, 7)), {
		word: 'beta',
		startColumn: 8,
		endColumn: 12,
	});
	assert.deepEqual(model.getWordAtPosition(position(1, 4)), {
		word: 'tail',
		startColumn: 1,
		endColumn: 5,
	});
});

test('TextModel delegates unusual-line-terminator state to ITextBuffer', () => {
	using model = new TextModel('alpha');
	model.applyOperations([{ range: new Range(1, 6, 1, 6), text: '\u2028omega' }]);

	assert.equal(model.mightContainUnusualLineTerminators(), true);
	model.removeUnusualLineTerminators();
	assert.equal(model.getValue(), 'alphaomega');
	assert.equal(model.mightContainUnusualLineTerminators(), false);

	model.undo();
	assert.equal(model.getValue(), 'alpha\u2028omega');
	assert.equal(model.mightContainUnusualLineTerminators(), true);
});

test('TextModel separates the editor snapshot iterator from versioned language snapshots', () => {
	using model = new TextModel(`\uFEFF${'x'.repeat(70_000)}`);
	const snapshot = model.createSnapshot(true);
	const versionedSnapshot = model.createVersionedSnapshot();

	assert.equal(isITextSnapshot(snapshot), true);
	const snapshotChunks: string[] = [];
	for (let chunk = snapshot.read(); chunk !== null; chunk = snapshot.read()) snapshotChunks.push(chunk);
	assert.equal(snapshotChunks.join(''), `\uFEFF${'x'.repeat(70_000)}`);
	model.setValue({
		read: (() => {
			const chunks: Array<string | null> = ['first\r\n', 'second', null];
			return () => chunks.shift() ?? null;
		})(),
	});
	assert.equal(model.getValue(), 'first\r\nsecond');
	assert.equal(model.canUndo(), false);
	assert.equal(versionedSnapshot.version, 1);
	assert.equal(versionedSnapshot.getText(), 'x'.repeat(70_000));
});

test('TextModel owns resolved indentation options and publishes exact changes', () => {
	using model = new TextModel('value', {
		tabSize: 2,
		indentSize: 'tabSize',
		insertSpaces: true,
		trimAutoWhitespace: false,
	});
	const events: unknown[] = [];
	using listener = model.onDidChangeOptions(event => events.push(event));

	assert.deepEqual(model.getFormattingOptions(), { tabSize: 2, insertSpaces: true });
	assert.equal(model.getOptions().originalIndentSize, 'tabSize');
	assert.equal(model.normalizeIndentation('\t value'), '   value');
	model.updateOptions({ tabSize: 4, indentSize: 2, insertSpaces: false, trimAutoWhitespace: true });
	assert.deepEqual(model.getFormattingOptions(), { tabSize: 2, insertSpaces: false });

	assert.deepEqual({
		tabSize: model.getOptions().tabSize,
		indentSize: model.getOptions().indentSize,
		originalIndentSize: model.getOptions().originalIndentSize,
		insertSpaces: model.getOptions().insertSpaces,
		trimAutoWhitespace: model.getOptions().trimAutoWhitespace,
		formatting: model.getFormattingOptions(),
		normalized: model.normalizeIndentation('    value'),
		events,
	}, {
		tabSize: 4,
		indentSize: 2,
		originalIndentSize: 2,
		insertSpaces: false,
		trimAutoWhitespace: true,
		formatting: { tabSize: 2, insertSpaces: false },
		normalized: '\t\tvalue',
		events: [{ tabSize: true, indentSize: false, insertSpaces: true, trimAutoWhitespace: true }],
	});
	model.updateOptions({ tabSize: 4 });
	assert.equal(events.length, 1);
});

test('TextModel detects indentation from its physical text buffer', () => {
	const spaces = new TextModel('root\n  child\n    grandchild');
	spaces.detectIndentation(false, 4);
	assert.equal(spaces.getOptions().insertSpaces, true);
	assert.equal(spaces.getOptions().tabSize, 2);
	assert.equal(spaces.getOptions().indentSize, 2);

	const tabs = new TextModel('root\n\tchild\n\t\tgrandchild');
	tabs.detectIndentation(true, 4);
	assert.equal(tabs.getOptions().insertSpaces, false);
	assert.equal(tabs.getOptions().tabSize, 4);
	assert.equal(tabs.getOptions().indentSize, 4);
});

test('TextModel exposes stable large-file and long-line policy', () => {
	const model = new TextModel(`${'x'.repeat(10_000)}\nshort`);
	assert.equal(model.isDominatedByLongLines(), true);
	assert.equal(model.isTooLargeForSyncing(), false);
	assert.equal(model.isTooLargeForTokenization(), false);
	assert.equal(model.isTooLargeForHeapOperation(), false);
});

test("TextModel reset replaces content and clears undo and redo history", () => {
	using model = new TextModel("initial");
	const snapshot = model.createVersionedSnapshot();
	const events: unknown[] = [];
	using listener = model.onDidChangeContent(change => events.push(change));
	model.applyOperations([{ range: range(0, 0, 0, 7), text: "edited" }]);
	model.undo();

	const reset = model.reset("next\r\nline");

	assert.ok(reset);
	assert.equal(model.getText(), "next\r\nline");
	assert.equal(model.length, 10);
	assert.equal(model.getEndOfLineSequence(), EndOfLineSequence.CRLF);
	assert.equal(model.canUndo(), false);
	assert.equal(model.canRedo(), false);
	assert.equal(model.undo(), undefined);
	assert.equal(model.redo(), undefined);
	assert.equal(snapshot.getText(), "initial");
	assert.equal(reset.reason, TextModelChangeReason.Reset);
	assert.equal(reset.changes.length, 1);
	assert.equal(events.length, 3);

	const eolReset = model.reset("next\nline");
	assert.ok(eolReset);
	assert.equal(model.getEndOfLineSequence(), EndOfLineSequence.LF);
	assert.equal(model.version, reset.version + 1);
	assert.equal(events.length, 4);
	const bomReset = model.reset("\uFEFFnext\nline");
	assert.ok(bomReset);
	assert.equal(model.getValue(EndOfLinePreference.TextDefined, true), "\uFEFFnext\nline");
	assert.equal(model.createSnapshot(true).read(), "\uFEFFnext\nline");
	assert.equal(events.length, 5);
	assert.equal(model.reset("\uFEFFnext\nline"), undefined);
	assert.equal(events.length, 5);
});

test('TextModel EOL changes preserve positions, history, and event identity', () => {
	using model = new TextModel('first\nsecond');
	using tracked = model.trackRange(range(1, 1, 1, 4), TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
	const events: Array<{ readonly eol: string; readonly isEolChange: boolean }> = [];
	using listener = model.onDidChangeContent(event => events.push(event));

	model.applyOperations([{ range: range(1, 0, 1, 1), text: 'S' }]);
	model.setEOL(EndOfLineSequence.CRLF);
	assert.equal(model.getText(), 'first\r\nSecond');
	assert.deepEqual(tracked.range, range(1, 1, 1, 4));
	assert.equal(events.at(-1)?.eol, '\r\n');
	assert.equal(events.at(-1)?.isEolChange, true);

	model.undo();
	assert.equal(model.getText(), 'first\nsecond');
	assert.equal(model.getEndOfLineSequence(), EndOfLineSequence.LF);
	model.redo();
	assert.equal(model.getText(), 'first\r\nSecond');
	assert.equal(model.getEndOfLineSequence(), EndOfLineSequence.CRLF);

	model.pushEOL(EndOfLineSequence.LF);
	assert.equal(model.getEOL(), '\n');
	model.undo();
	assert.equal(model.getEOL(), '\r\n');
	model.redo();
	assert.equal(model.getEOL(), '\n');
});

test("TextModel applies unordered edits against one atomic snapshot", () => {
	using model = new TextModel("alpha\nbeta\ngamma");
	const events: unknown[] = [];
	using listener = model.onDidChangeContent(event => events.push(event));

	const change = model.applyOperations([
		{ range: range(2, 0, 2, 5), text: "G" },
		{ range: range(0, 5, 0, 5), text: "!" },
		{ range: range(1, 0, 1, 4), text: "B\r\nB2" },
	]);

	assert.deepEqual({
		text: model.getText(),
		version: model.version,
		change,
		eventCount: events.length,
	}, {
		text: "alpha!\nB\nB2\nG",
		version: 2,
		change: {
			version: 2,
			transactionId: 1,
			reason: TextModelChangeReason.Edit,
			eol: '\n',
			isEolChange: false,
			detailedReasons: [],
			resultingSelection: null,
			changes: [
				{
					range: range(0, 5, 0, 5),
					rangeOffset: 5,
					rangeLength: 0,
					text: "!",
				},
				{
					range: range(1, 0, 1, 4),
					rangeOffset: 6,
					rangeLength: 4,
					text: "B\nB2",
				},
				{
					range: range(2, 0, 2, 5),
					rangeOffset: 11,
					rangeLength: 5,
					text: "G",
				},
			],
		},
		eventCount: 1,
	});
});

test("TextModel rejects overlapping edits without mutating", () => {
	using model = new TextModel("abcdef");

	assert.throws(() => model.applyEdits([
		{ range: range(0, 1, 0, 4), text: "x" },
		{ range: range(0, 3, 0, 5), text: "y" },
	]), /must not overlap/);
	assert.throws(() => model.applyEdits([
		{ range: Range.fromPositions(position(0, 2)), text: "x" },
		{ range: Range.fromPositions(position(0, 2)), text: "y" },
	]), /must not overlap/);
	assert.deepEqual({
		text: model.getText(),
		version: model.version,
		canUndo: model.canUndo(),
	}, {
		text: "abcdef",
		version: 1,
		canUndo: false,
	});
});

test("TextModel undo and redo preserve transaction boundaries", () => {
	using model = new TextModel("abcdef");
	const reasons: TextModelChangeReason[] = [];
	using listener = model.onDidChangeContent(event => reasons.push(event.reason));

	model.applyOperations([
		{ range: range(0, 1, 0, 3), text: "LONG" },
		{ range: range(0, 5, 0, 6), text: "" },
	]);
	const edited = model.getText();
	model.undo();
	const undone = model.getText();
	model.redo();

	assert.deepEqual({
		edited,
		undone,
		redone: model.getText(),
		version: model.version,
		canUndo: model.canUndo(),
		canRedo: model.canRedo(),
		reasons,
	}, {
		edited: "aLONGde",
		undone: "abcdef",
		redone: "aLONGde",
		version: 4,
		canUndo: true,
		canRedo: false,
		reasons: [
			TextModelChangeReason.Edit,
			TextModelChangeReason.Undo,
			TextModelChangeReason.Redo,
		],
	});
});

test('TextModel alternative version follows document states through undo and redo', () => {
	using model = new TextModel('a');
	assert.equal(model.getAlternativeVersionId(), 1);

	model.applyOperations([{ range: range(0, 1, 0, 1), text: 'b' }]);
	const firstEditedAlternativeVersion = model.getAlternativeVersionId();
	model.applyOperations([{ range: range(0, 2, 0, 2), text: 'c' }]);
	const secondEditedAlternativeVersion = model.getAlternativeVersionId();
	assert.deepEqual([firstEditedAlternativeVersion, secondEditedAlternativeVersion], [2, 3]);

	model.undo();
	assert.deepEqual({ text: model.getValue(), version: model.getVersionId(), alternativeVersion: model.getAlternativeVersionId() }, {
		text: 'ab',
		version: 4,
		alternativeVersion: firstEditedAlternativeVersion,
	});
	model.undo();
	assert.deepEqual({ text: model.getValue(), version: model.getVersionId(), alternativeVersion: model.getAlternativeVersionId() }, {
		text: 'a',
		version: 5,
		alternativeVersion: 1,
	});
	model.redo();
	assert.equal(model.getAlternativeVersionId(), firstEditedAlternativeVersion);
	model.redo();
	assert.equal(model.getAlternativeVersionId(), secondEditedAlternativeVersion);

	model.reset('reset');
	assert.equal(model.getAlternativeVersionId(), model.getVersionId());
});

test("TextModel clears redo on a new edit and ignores exact no-ops", () => {
	using model = new TextModel("abc");
	let eventCount = 0;
	using listener = model.onDidChangeContent(() => eventCount += 1);

	assert.equal(model.applyOperations([
		{ range: range(0, 0, 0, 3), text: "abc" },
	]), undefined);
	model.applyOperations([
		{ range: range(0, 0, 0, 1), text: "A" },
	]);
	model.undo();
	model.applyOperations([
		{ range: range(0, 1, 0, 2), text: "B" },
	]);

	assert.deepEqual({
		text: model.getText(),
		version: model.version,
		canRedo: model.canRedo(),
		eventCount,
	}, {
		text: "aBc",
		version: 4,
		canRedo: false,
		eventCount: 3,
	});
});

test("TextModel validates positions and rejects access after disposal", () => {
	const model = new TextModel("abc");

	assert.throws(() => model.offsetAt(position(1, 0)), /lineNumber/);
	assert.throws(() => model.offsetAt(position(0, 4)), /column/);
	assert.throws(() => model.positionAt(4), /offset/);
	model.dispose();
	assert.throws(() => model.getText(), /already disposed/);
});

test("TextModel owns attached editor count and visible-line handles", () => {
	using model = new TextModel("one\ntwo");
	let attachedChanges = 0;
	using listener = model.onDidChangeAttached(() => attachedChanges++);
	const first = model.onBeforeAttached();
	const second = model.onBeforeAttached();
	assert.equal(model.isAttachedToEditor(), true);
	assert.equal(model.getAttachedEditorCount(), 2);
	assert.equal(attachedChanges, 1);
	first.setVisibleLines([{ startLineNumber: 1, endLineNumber: 2 }], true);
	assert.throws(() => first.setVisibleLines([{ startLineNumber: 0, endLineNumber: 1 }], false), /valid model line ranges/);
	model.onBeforeDetached(first);
	assert.equal(attachedChanges, 1);
	model.onBeforeDetached(second);
	assert.equal(model.isAttachedToEditor(), false);
	assert.equal(attachedChanges, 2);
	assert.throws(() => second.setVisibleLines([], false), /not attached/);
});

test("TextModel owns VS Code tracked-range identifiers", () => {
	using model = new TextModel("abc");
	const id = model._setTrackedRange(null, new Range(1, 2, 1, 3), TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
	assert.deepEqual(model._getTrackedRange(id), new Range(1, 2, 1, 3));
	model.applyOperations([{ range: new Range(1, 1, 1, 1), text: "X" }]);
	assert.deepEqual(model._getTrackedRange(id), new Range(1, 3, 1, 4));
	assert.equal(model._setTrackedRange(id, null, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges), null);
	assert.equal(model._getTrackedRange(id), null);
});

test("TextModel commits history before reentrant change listeners run", () => {
	using model = new TextModel("abc");
	const versions: number[] = [];
	using listener = model.onDidChangeContent(event => {
		versions.push(event.version);
		if (event.version === 2) {
			model.applyOperations([
				{ range: range(0, 1, 0, 2), text: "B" },
			]);
		}
	});

	model.applyOperations([
		{ range: range(0, 0, 0, 1), text: "A" },
	]);
	model.undo();
	const afterFirstUndo = model.getText();
	model.undo();

	assert.deepEqual({
		afterFirstUndo,
		afterSecondUndo: model.getText(),
		versions,
	}, {
		afterFirstUndo: "Abc",
		afterSecondUndo: "abc",
		versions: [2, 3, 4, 5],
	});
});
