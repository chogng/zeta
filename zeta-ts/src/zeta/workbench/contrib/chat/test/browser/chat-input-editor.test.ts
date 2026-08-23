import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { ChatInputEditorRegistry, type ChatInputEditorOptions, type IChatInputEditor } from "../../../../../workbench/contrib/chat/browser/input/chatInputEditor.js";
import { SlashCommandCatalog } from "../../../../../workbench/contrib/chat/common/slashCommands.js";
import { h } from "../../../../../base/browser/dom.js";

const slashCommands = new SlashCommandCatalog([], []);

test("Chat input editor fallback owns multiline change and submit gestures", () => {
	const dom = new JSDOM("<!doctype html><body><div id='host'></div></body>");
	const container = dom.window.document.querySelector<HTMLElement>("#host");
	assert.ok(container);
	const registry = new ChatInputEditorRegistry();
	using editor = registry.create({
		container,
		placeholder: "Ask Zeta",
		ariaLabel: "Chat message",
		slashCommands,
	});
	const changes: string[] = [];
	let submissions = 0;
	using changeListener = editor.onDidChange((value) => changes.push(value));
	using submitListener = editor.onDidSubmit(() => submissions++);
	const textarea = editor.element;
	assert.ok(textarea instanceof dom.window.HTMLTextAreaElement);
	assert.equal(registry.activeProviderId, "textarea");
	assert.equal(textarea.getAttribute("aria-label"), "Chat message");
	assert.equal(textarea.getAttribute("placeholder"), "Ask Zeta");

	editor.value = "First draft";
	assert.deepEqual(changes, ["First draft"]);
	textarea.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key: "Enter",
		shiftKey: true,
	}));
	assert.equal(submissions, 0);
	const submitEvent = new dom.window.KeyboardEvent("keydown", {
		bubbles: true,
		cancelable: true,
		key: "Enter",
	});
	textarea.dispatchEvent(submitEvent);
	assert.equal(submitEvent.defaultPrevented, true);
	assert.equal(submissions, 1);

	editor.focus();
	assert.equal(dom.window.document.activeElement, textarea);
	editor.layout();
	editor.dispose();
	assert.equal(container.childElementCount, 0);
	dom.window.close();
});

test("Chat input editor registry selects and releases a product provider", () => {
	const dom = new JSDOM("<!doctype html><body><div id='host'></div></body>");
	const container = dom.window.document.querySelector<HTMLElement>("#host");
	assert.ok(container);
	const registry = new ChatInputEditorRegistry();
	const created: ChatInputEditorOptions[] = [];
	const provider = {
		id: "rich-editor",
		create: (options: ChatInputEditorOptions) => {
			created.push(options);
			return new FakeChatInputEditor(options);
		},
	};
	using registration = registry.register(provider);
	assert.equal(registry.activeProviderId, "rich-editor");
	assert.throws(() => registry.register(provider), /already registered/);

	using editor = registry.create({
		container,
		placeholder: "Prompt",
		ariaLabel: "Prompt editor",
		slashCommands,
	});
	assert.ok(editor instanceof FakeChatInputEditor);
	assert.equal(created.length, 1);
	assert.equal(editor.element.parentElement, container);

	registration.dispose();
	assert.equal(registry.activeProviderId, "textarea");
	dom.window.close();
});

class FakeChatInputEditor extends DisposableOwner implements IChatInputEditor {
	readonly element: HTMLDivElement;
	private readonly _onDidChange = this.own(new Emitter<string>());
	private readonly _onDidSubmit = this.own(new Emitter<void>());
	readonly onDidChange: Event<string> = this._onDidChange.event;
	readonly onDidSubmit: Event<void> = this._onDidSubmit.event;
	value = "";

	constructor(options: ChatInputEditorOptions) {
		super();
		this.element = h(options.container.ownerDocument, "div");
		options.container.append(this.element);
		this.defer(() => this.element.remove());
	}

	focus(): void {
		this.element.focus();
	}

	layout(): void {}
}
