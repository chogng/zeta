import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const editorRoot = resolve(findDesktopRoot(import.meta.dirname), "src/zeta/editor");

test("flat Stanza domain exposes public entrypoints and mode bundles", () => {
	for (const entrypoint of ["editor.api.ts", "editor.code.all.ts", "editor.academic.all.ts", "editor.all.ts", "editor.main.ts", "editor.worker.start.ts"]) {
		assert.equal(exists(join(editorRoot, entrypoint)), true, entrypoint);
	}
	assert.equal(exists(join(editorRoot, "alpha")), false, "alpha directory");
	assert.equal(exists(join(editorRoot, "gama")), false, "gama directory");
	for (const retiredEntrypoint of ["stanza.api.ts", "stanza.code.all.ts", "stanza.academic.all.ts", "stanza.all.ts", "stanza.main.ts", "stanza.worker.start.ts"]) {
		assert.equal(exists(join(editorRoot, retiredEntrypoint)), false, retiredEntrypoint);
	}
});

test("public Stanza entrypoints retain distinct API, contribution, main, and worker roles", () => {
	const api = readFileSync(join(editorRoot, "editor.api.ts"), "utf8");
	const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
	const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
	const all = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
	const main = readFileSync(join(editorRoot, "editor.main.ts"), "utf8");
	const worker = readFileSync(join(editorRoot, "editor.worker.start.ts"), "utf8");
	const analysisWorker = readFileSync(join(editorRoot, "browser/language/syntaxWorkerMain.ts"), "utf8");
	const completionWorker = readFileSync(join(editorRoot, "browser/language/languageCompletionWorkerMain.ts"), "utf8");
	assert.match(api, /TextModel/u);
	assert.match(api, /TextModelBlockTree/u);
	assert.match(api, /TextModelGroup/u);
	assert.doesNotMatch(api, /TextModelStructure/u);
	assert.doesNotMatch(api, /\bDocumentModel\b/u);
	assert.doesNotMatch(api, /workbench|browser|contrib/u);
	assert.match(codeBundle, /editor\.all/u);
	assert.doesNotMatch(codeBundle, /contrib\//u);
	assert.doesNotMatch(codeBundle, /contrib\/academic/u);
	assert.doesNotMatch(academicBundle, /editor\.all/u);
	assert.match(academicBundle, /contrib\/documentEditor\.contribution/u);
	assert.doesNotMatch(academicBundle, /workbench|academicEditor\.contribution/u);
	assert.match(all, /contrib\/codeEditorPart\.contribution/u);
	assert.match(all, /quickAccess\/browser\/quickAccessController/u);
	assert.doesNotMatch(all, /editor\.(?:code|academic)\.all/u);
	assert.match(main, /import "\.\/editor\.all\.js"/u);
	assert.match(main, /export \* from "\.\/editor\.api\.js"/u);
	assert.match(worker, /StanzaWorkerPort/u);
	assert.match(worker, /export function start/u);
	assert.match(analysisWorker, /languageWorker\.start/u);
	assert.match(completionWorker, /languageWorker\.start/u);
});

function exists(file: string): boolean {
	try {
		return statSync(file).isFile();
	} catch {
		return false;
	}
}
