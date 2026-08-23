import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM, type DOMWindow } from "jsdom";
import { scheduleAtNextAnimationFrame } from "../../browser/scheduler.js";
import { AriaLiveRegion, setAriaAttribute, setRole } from "../../browser/ui/aria/aria.js";

test("ARIA helpers set, preserve false, and remove semantic attributes", () => {
	const dom = new JSDOM("<!doctype html><body><button></button></body>");
	const button = dom.window.document.querySelector("button");
	assert.ok(button);

	setRole(button, "menuitem");
	setAriaAttribute(button, "expanded", false);
	setAriaAttribute(button, "controls", "menu");
	assert.equal(button.getAttribute("role"), "menuitem");
	assert.equal(button.getAttribute("aria-expanded"), "false");
	assert.equal(button.getAttribute("aria-controls"), "menu");

	setRole(button, undefined);
	setAriaAttribute(button, "controls", undefined);
	assert.equal(button.hasAttribute("role"), false);
	assert.equal(button.hasAttribute("aria-controls"), false);
	dom.window.close();
});

test("AriaLiveRegion announces repeated status and alert messages", async () => {
	const dom = new JSDOM("<!doctype html><body></body>", {
		pretendToBeVisual: true,
	});
	const region = new AriaLiveRegion(dom.window.document);

	region.status("Ready");
	await nextAnimationFrame(dom.window);
	assert.deepEqual(
		[...dom.window.document.querySelectorAll(".zeta-aria-status")]
			.map((element) => element.textContent),
		["Ready", ""],
	);

	region.status("Ready");
	await nextAnimationFrame(dom.window);
	assert.deepEqual(
		[...dom.window.document.querySelectorAll(".zeta-aria-status")]
			.map((element) => element.textContent),
		["", "Ready"],
	);

	region.alert("Failed");
	await nextAnimationFrame(dom.window);
	const alert = dom.window.document.querySelector(".zeta-aria-alert");
	assert.equal(alert?.getAttribute("role"), "alert");
	assert.equal(alert?.getAttribute("aria-atomic"), "true");
	assert.equal(alert?.textContent, "Failed");

	region.clear();
	assert.equal(
		dom.window.document.querySelector(".zeta-aria-live")?.textContent,
		"",
	);
	region.dispose();
	assert.equal(
		dom.window.document.querySelector(".zeta-aria-live"),
		null,
	);
	dom.window.close();
});

function nextAnimationFrame(targetWindow: DOMWindow): Promise<void> {
	return new Promise((resolve) => {
		scheduleAtNextAnimationFrame(targetWindow as unknown as Window, resolve);
	});
}
