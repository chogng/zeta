import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	InputEvent: browserEnvironment.window.InputEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { ChatInputEditor } = await import("../../browser/input/stanzaChatInputEditor.js");
const { DesktopSlashCommands, SlashCommandCatalog } = await import("../../common/slashCommands.js");

test.after(() => browserEnvironment.window.close());

test("Stanza Chat input completes slash commands before submitting", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using editor = new ChatInputEditor({ container, placeholder: "Ask Zeta", ariaLabel: "Chat message", slashCommands: new SlashCommandCatalog(DesktopSlashCommands, []) });
	let submissions = 0;
	using submitListener = editor.onDidSubmit(() => submissions += 1);
	const input = requiredElement<HTMLTextAreaElement>(editor.element, ".stanza-editor-input");

	input.dispatchEvent(beforeInputEvent(dom.window, "/"));
	await waitFor(() => completionLabels(editor.element).length === 2);
	assert.deepEqual(completionLabels(editor.element), ["/new", "/history"]);
	assert.equal(editor.element.querySelector(".stanza-editor")?.classList.contains("stanza-editor-embedded"), true);
	assert.equal(editor.element.querySelector(".stanza-editor")?.classList.contains("stanza-editor-focus-owner-host"), true);
	assert.equal(editor.element.querySelector(".stanza-editor")?.classList.contains("word-wrapped"), true);
	assert.equal(editor.element.querySelector<HTMLElement>(".stanza-editor-line-number")?.style.display, "");

	input.dispatchEvent(beforeInputEvent(dom.window, "n"));
	await waitFor(() => completionLabels(editor.element).length === 1);
	assert.deepEqual(completionLabels(editor.element), ["/new"]);

	const accept = keyboardEvent(dom.window, "Enter");
	input.dispatchEvent(accept);
	assert.equal(accept.defaultPrevented, true);
	assert.equal(editor.value, "/new ");
	assert.equal(submissions, 0);

	const submit = keyboardEvent(dom.window, "Enter");
	input.dispatchEvent(submit);
	assert.equal(submit.defaultPrevented, true);
	assert.equal(submissions, 1);
	dom.window.close();
});

test("Stanza Chat input discovers dynamically projected Skill commands", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	const catalog = new SlashCommandCatalog(DesktopSlashCommands, []);
	using editor = new ChatInputEditor({ container, placeholder: "Ask Zeta", ariaLabel: "Chat message", slashCommands: catalog });
	catalog.setSkillCommands([{
		name: "commit",
		description: "Draft a commit message",
		source: "user",
		skill: { id: { source: "user:skill-source:test", name: "commit" }, version: { type: "pinnedDigest", digest: "sha256:commit" } },
	}]);
	const input = requiredElement<HTMLTextAreaElement>(editor.element, ".stanza-editor-input");

	input.dispatchEvent(beforeInputEvent(dom.window, "/"));
	await waitFor(() => completionLabels(editor.element).length === 3);

	assert.deepEqual(completionLabels(editor.element), ["/new", "/history", "/commit"]);
	dom.window.close();
});

test("Stanza Chat input restores message behavior when the slash is deleted", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using editor = new ChatInputEditor({ container, placeholder: "Ask Zeta", ariaLabel: "Chat message", slashCommands: new SlashCommandCatalog(DesktopSlashCommands, []) });
	const changes: string[] = [];
	using changeListener = editor.onDidChange(value => changes.push(value));
	const input = requiredElement<HTMLTextAreaElement>(editor.element, ".stanza-editor-input");

	input.dispatchEvent(beforeInputEvent(dom.window, "/"));
	await waitFor(() => completionLabels(editor.element).length > 0);
	input.dispatchEvent(beforeInputEvent(dom.window, "x"));
	await waitFor(() => editor.element.querySelector(".stanza-editor-completion.visible") === null);
	input.dispatchEvent(beforeInputEvent(dom.window, null, "deleteContentBackward"));
	await waitFor(() => completionLabels(editor.element).length > 0);
	input.dispatchEvent(beforeInputEvent(dom.window, null, "deleteContentBackward"));
	await waitFor(() => editor.element.querySelector(".stanza-editor-completion.visible") === null);

	assert.equal(editor.value, "");
	assert.deepEqual(changes, ["/", "/x", "/", ""]);
	const placeholder = requiredElement<HTMLElement>(editor.element, ".stanza-editor-placeholder-text");
	assert.equal(placeholder.hidden, false);
	assert.equal(placeholder.style.top, "0px");
	assert.equal(placeholder.style.left, "0px");
	dom.window.close();
});

test("Stanza Chat input starts at the InputPart default height and still grows with content", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	dom.window.HTMLCanvasElement.prototype.getContext = () => null;
	const container = requiredElement<HTMLElement>(dom.window.document, "main");
	using editor = new ChatInputEditor({ container, placeholder: "Ask Zeta", ariaLabel: "Chat message", slashCommands: new SlashCommandCatalog(DesktopSlashCommands, []) });

	assert.equal(editor.element.style.height, "106px");
	editor.value = Array.from({ length: 12 }, (_, index) => `Line ${index + 1}`).join("\n");
	assert.equal(editor.element.style.height, "240px");
	dom.window.close();
});

function completionLabels(root: ParentNode): string[] {
	return [...root.querySelectorAll<HTMLElement>(".stanza-editor-completion-label")].map(element => element.textContent ?? "");
}

function beforeInputEvent(targetWindow: typeof browserEnvironment.window, data: string | null, inputType = "insertText"): InputEvent {
	return new targetWindow.InputEvent("beforeinput", { bubbles: true, cancelable: true, inputType, data }) as unknown as InputEvent;
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string): KeyboardEvent {
	return new targetWindow.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key }) as unknown as KeyboardEvent;
}

async function waitFor(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 30; attempt += 1) {
		if (predicate()) return;
		await new Promise<void>(resolve => setTimeout(resolve, 0));
	}
	assert.fail("Timed out waiting for Stanza Chat input state");
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}
