import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { addDisposableListener, h, isElement, isHTMLElement, isNode, stopEvent, svg } from "../../browser/dom.js";

test("disposable DOM listeners detach deterministically", () => {
	const target = new EventTarget();
	let calls = 0;
	const registration = addDisposableListener(
		target,
		"change",
		() => calls++,
	);

	target.dispatchEvent(new Event("change"));
	registration.dispose();
	registration.dispose();
	target.dispatchEvent(new Event("change"));

	assert.equal(calls, 1);
});

test("disposable DOM listeners retain their registered capture mode", () => {
	const ownerWindow = new JSDOM("<!doctype html><body><button></button></body>").window;
	const button = ownerWindow.document.querySelector("button")!;
	const options = { capture: true };
	let calls = 0;
	const registration = addDisposableListener(button, "click", () => calls++, options);

	options.capture = false;
	registration.dispose();
	button.dispatchEvent(new ownerWindow.MouseEvent("click", { bubbles: true }));

	assert.equal(calls, 0);
});

test("DOM guards validate nodes from their owning window", () => {
	const firstDocument = new JSDOM("<!doctype html><body></body>").window.document;
	const secondDocument = new JSDOM("<!doctype html><body></body>").window.document;
	const element = h(secondDocument, "div");
	const svgElement = svg(secondDocument, "svg");

	assert.deepEqual({
		firstDocument: isNode(firstDocument),
		secondDocument: isNode(secondDocument),
		element: isElement(element),
		htmlElement: isHTMLElement(element),
		svgElement: isElement(svgElement),
		svgHtmlElement: isHTMLElement(svgElement),
		structuralLookalike: isNode({
			nodeType: 1,
			namespaceURI: "http://www.w3.org/1999/xhtml",
		}),
	}, {
		firstDocument: true,
		secondDocument: true,
		element: true,
		htmlElement: true,
		svgElement: true,
		svgHtmlElement: false,
		structuralLookalike: false,
	});
});

test("stopEvent prevents native behavior and propagation", () => {
	const event = new Event("submit", {
		bubbles: true,
		cancelable: true,
	});
	let propagated = false;
	const target = new EventTarget();
	target.addEventListener("submit", (next) =>
		stopEvent(next, { immediate: true }),
	);
	target.addEventListener("submit", () => {
		propagated = true;
	});

	const accepted = target.dispatchEvent(event);

	assert.equal(accepted, false);
	assert.equal(event.defaultPrevented, true);
	assert.equal(propagated, false);
});
