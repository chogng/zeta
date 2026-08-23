import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../../base/common/event.js";
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationService } from "../../../configuration/common/configurationService.js";
import { ConfigurationsRegistry } from "../../../configuration/common/configurationRegistry.js";
import { WorkbenchObjectTree, type ResourceOpenEvent } from "../../browser/listService.js";
import { ListConfiguration, type ListOpenMode } from "../../common/listConfiguration.js";
import { h } from "../../../../base/browser/dom.js";

interface TestItem {
	readonly id: string;
}

test("Platform List owns and validates the shared open-mode configuration", () => {
	assert.equal(ConfigurationsRegistry.owns(ListConfiguration.openMode), true);
	assert.equal(ListConfiguration.openMode.defaultValue, "singleClick");
	assert.equal(ListConfiguration.openMode.parse("doubleClick"), "doubleClick");
	assert.throws(() => ListConfiguration.openMode.parse("hover"), /Unknown list open mode/);
});

test("WorkbenchObjectTree derives preview, pinned, and side-by-side open intent", async () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	using configuration = new TestConfigurationService();
	using tree = new WorkbenchObjectTree<TestItem>(dom.window.document.body, {
		ariaLabel: "Resources",
		configurationService: configuration,
		modelOptions: { identityProvider: { getId: (item) => item.id } },
		renderElement: (item) => {
			const label = h(dom.window.document, "span");
			label.textContent = item.id;
			return label;
		},
	});
	const first = { id: "readme" };
	const second = { id: "source" };
	tree.setChildren([{ element: first }, { element: second }]);
	const opens: ResourceOpenEvent<TestItem>[] = [];
	using listener = tree.onDidOpen((event) => opens.push(event));
	const firstRow = tree.element.querySelector<HTMLElement>('[data-tree-id="readme"]')!;

	firstRow.dispatchEvent(mouse(dom, "click", { detail: 1 }));
	assertOpen(opens[0], first, false, true, false);

	firstRow.dispatchEvent(mouse(dom, "click", { detail: 1, ctrlKey: true }));
	assertOpen(opens[1], first, false, true, true);

	firstRow.dispatchEvent(mouse(dom, "auxclick", { detail: 1, button: 1 }));
	assertOpen(opens[2], first, true, true, false);

	firstRow.dispatchEvent(mouse(dom, "click", { detail: 2 }));
	assert.equal(opens.length, 3);
	firstRow.dispatchEvent(mouse(dom, "dblclick", { detail: 2 }));
	assertOpen(opens[3], first, true, false, false);

	tree.setFocus("readme");
	tree.element.dispatchEvent(keyboard(dom, "Enter", { metaKey: true }));
	assertOpen(opens[4], first, true, false, true);

	tree.setSelection(["source"], keyboard(dom, "ArrowDown"));
	assertOpen(opens[5], second, false, true, false);

	await configuration.updateValue(ListConfiguration.openMode, "doubleClick");
	firstRow.dispatchEvent(mouse(dom, "click", { detail: 1 }));
	assert.equal(opens.length, 6);
	dom.window.close();
});

function assertOpen(event: ResourceOpenEvent<TestItem> | undefined, element: TestItem, pinned: boolean, preserveFocus: boolean, sideBySide: boolean): void {
	assert.equal(event?.element, element);
	assert.deepEqual(event?.editorOptions, { pinned, preserveFocus });
	assert.equal(event?.sideBySide, sideBySide);
}

function mouse(dom: JSDOM, type: string, init: MouseEventInit): MouseEvent {
	return new dom.window.MouseEvent(type, { bubbles: true, ...init }) as unknown as MouseEvent;
}

function keyboard(dom: JSDOM, key: string, init: KeyboardEventInit = {}): KeyboardEvent {
	return new dom.window.KeyboardEvent("keydown", { bubbles: true, key, ...init }) as unknown as KeyboardEvent;
}

class TestConfigurationService implements IConfigurationService {
	private readonly changes = new Emitter<IConfigurationChangeEvent>();
	private listOpenMode: ListOpenMode = "singleClick";

	readonly onDidChangeConfiguration = this.changes.event;

	getValue<T>(key: IConfigurationKey<T>): T {
		return (key === ListConfiguration.openMode ? this.listOpenMode : key.defaultValue) as T;
	}

	async updateValue<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
		if (key === ListConfiguration.openMode) this.listOpenMode = value as ListOpenMode;
		this.changes.fire({
			keys: new Set([key.key]),
			affectsConfiguration: (candidate) => candidate.key === key.key,
		});
	}

	async resetValue<T>(key: IConfigurationKey<T>): Promise<void> {
		if (key === ListConfiguration.openMode) this.listOpenMode = ListConfiguration.openMode.defaultValue;
	}

	async reload(): Promise<void> {}

	[Symbol.dispose](): void {
		this.changes.dispose();
	}
}
