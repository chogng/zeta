import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { h } from "../../../../../base/browser/dom.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	PointerEvent: browserEnvironment.window.MouseEvent,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { ServiceContainer } = await import("../../../../../platform/instantiation/common/instantiation.js");
const { IConfigurationService } = await import("../../../../../platform/configuration/common/configurationService.js");
const { ILayoutService } = await import("../../../../../platform/layout/common/layoutService.js");
const { WorkbenchContributionsRegistry, WorkbenchPhase } = await import("../../../../../workbench/common/contributions.js");
const { SashConfiguration } = await import("../../../../../workbench/contrib/sash/common/sash.js");
const { SashSettingsController } = await import("../../../../../workbench/contrib/sash/browser/sash.js");
const { WorkbenchConfigurationService } = await import("../../../../../workbench/services/configuration/browser/configurationService.js");
await import("../../../../../workbench/contrib/sash/browser/sash.contribution.js");

type LayoutService = import("../../../../../platform/layout/common/layoutService.js").ILayoutService;

test("Sash configuration validates its public range", () => {
	assert.equal(SashConfiguration.size.defaultValue, 4);
	assert.equal(SashConfiguration.hoverDelay.defaultValue, 300);
	assert.equal(SashConfiguration.size.parse(1), 1);
	assert.equal(SashConfiguration.size.parse(20), 20);
	assert.equal(SashConfiguration.hoverDelay.parse(0), 0);
	assert.equal(SashConfiguration.hoverDelay.parse(2_000), 2_000);
	assert.throws(() => SashConfiguration.size.parse(0), /between 1 and 20/);
	assert.throws(
		() => SashConfiguration.hoverDelay.parse(2_001),
		/between 0 and 2000/,
	);
});

test("Sash settings controller projects live configuration and restores styles", async () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const root = dom.window.document.querySelector("main");
	assert.ok(root);
	const sash = appendSash(root);
	using configuration = new WorkbenchConfigurationService();

	{
		using controller = new SashSettingsController(configuration, root);
		assertSashStyles(sash, "4px", "4px", "300ms");

		await configuration.updateValue(SashConfiguration.size, 12);
		await configuration.updateValue(SashConfiguration.hoverDelay, 0);
		assertSashStyles(sash, "12px", "8px", "0ms");

		await configuration.updateValue(SashConfiguration.size, 1);
		assertSashStyles(sash, "4px", "1px", "0ms");
	}

	assertSashStyles(sash, "", "", "");
	dom.window.close();
});

test("Sash contribution starts after restoration", () => {
	const dom = new JSDOM("<!doctype html><body><main></main></body>");
	const root = dom.window.document.querySelector("main");
	assert.ok(root);
	const sash = appendSash(root);
	using configuration = new WorkbenchConfigurationService();
	const services = new ServiceContainer();
	services.registerInstance(IConfigurationService, configuration);
	services.registerInstance(ILayoutService, { mainContainer: root } as LayoutService);

	using host = WorkbenchContributionsRegistry.createHost(services);
	host.advance(WorkbenchPhase.BlockRestore);
	assertSashStyles(sash, "", "", "");
	host.advance(WorkbenchPhase.AfterRestored);
	assertSashStyles(sash, "4px", "4px", "300ms");
	host.dispose();
	assertSashStyles(sash, "", "", "");
	dom.window.close();
});

function assertSashStyles(
	sash: HTMLElement,
	dragAreaSize: string,
	hoverFeedbackSize: string,
	hoverDelay: string,
): void {
	const targetWindow = sash.ownerDocument.defaultView;
	assert.ok(targetWindow);
	const style = targetWindow.getComputedStyle(sash);
	assert.equal(
		style.getPropertyValue("--zeta-sash-drag-area-size"),
		dragAreaSize,
	);
	assert.equal(
		style.getPropertyValue("--zeta-sash-hover-feedback-size"),
		hoverFeedbackSize,
	);
	assert.equal(
		style.getPropertyValue("--zeta-sash-hover-delay"),
		hoverDelay,
	);
}

function appendSash(root: HTMLElement): HTMLDivElement {
	const sash = h(root.ownerDocument, "div");
	sash.className = "zeta-sash";
	root.append(sash);
	return sash;
}
