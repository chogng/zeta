import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { registerEditorContribution } from "../../browser/editorExtensions.js";
import { getEditorContributions } from "../../browser/editorExtensions.js";
import { TriggerInlineEditCommandsRegistry } from '../../browser/triggerInlineEditCommandsRegistry.js';

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
Object.defineProperty(globalThis, "window", { configurable: true, value: browserEnvironment.window });
Object.defineProperty(globalThis, "document", { configurable: true, value: browserEnvironment.window.document });

test.after(() => browserEnvironment.window.close());

test("editor contributions retain bundle registration order and stable identity", async () => {
	const before = getEditorContributions().map(contribution => contribution.id);
	assert.equal(before.includes("editor.contrib.find"), false);

	await import("../../contrib/find/browser/find.contribution.js");
	const after = getEditorContributions().map(contribution => contribution.id);
	assert.deepEqual(after, [...before, "editor.contrib.find"]);
	const contribution = getEditorContributions().find(candidate => candidate.id === "editor.contrib.find");
	assert.ok(contribution);
	assert.doesNotThrow(() => contribution.install?.({ kind: "document" } as never));

	assert.throws(() => registerEditorContribution({ id: "editor.contrib.find", install() {} }), /Duplicate editor contribution/);
	assert.deepEqual(getEditorContributions().map(contribution => contribution.id), after);
});

test("Code bundle explicitly registers independently selectable editor capabilities", async () => {
	await import("../../editor.code.all.js");
	const ids = new Set(getEditorContributions().map(contribution => contribution.id));
	for (const id of [
		"editor.contrib.bracketMatching",
		"editor.contrib.codeAction",
		"editor.contrib.comment",
		"editor.contrib.dropOrPasteInto",
		"editor.contrib.folding",
		"editor.contrib.format",
		"editor.contrib.gotoSymbol",
		"editor.contrib.hover",
		"editor.contrib.multicursor",
		"editor.contrib.rename",
		"editor.contrib.wordHighlighter",
	]) {
		assert.equal(ids.has(id), true, id);
	}
	assert.equal(ids.has("editor.contrib.documentFormatting"), false);
	const triggerCommands = new Set(TriggerInlineEditCommandsRegistry.getRegisteredCommands());
	for (const id of [
		'editor.action.removeBrackets',
		'editor.action.commentLine',
		'editor.action.blockComment',
		'editor.action.joinLines',
		'editor.action.rename',
		'editor.action.transpose',
	]) assert.equal(triggerCommands.has(id), true, id);
});

test('Inline edit trigger command metadata validates IDs and deduplicates registrations', () => {
	const id = 'editor.test.triggerInlineEdit';
	TriggerInlineEditCommandsRegistry.registerCommand(id);
	TriggerInlineEditCommandsRegistry.registerCommand(id);
	assert.equal(TriggerInlineEditCommandsRegistry.getRegisteredCommands().filter(candidate => candidate === id).length, 1);
	assert.throws(() => TriggerInlineEditCommandsRegistry.registerCommand(''), /non-empty string/);
});
