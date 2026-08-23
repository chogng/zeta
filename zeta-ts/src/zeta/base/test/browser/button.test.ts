import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Button } from "../../browser/ui/button/button.js";
import { setHoverDelegate, type IManagedHover } from "../../browser/ui/hover/hoverDelegate.js";

test("Button only installs a Hover for an explicit title", () => {
	const dom = new JSDOM("<!doctype html><body></body>");
	const contents: unknown[] = [];
	using delegateRegistration = setHoverDelegate({
		setupHover(options) {
			contents.push(options.content);
			return managedHover();
		},
	});
	using unlabeledHoverButton = new Button(dom.window.document.body, {
		label: "Save",
	});
	using titledButton = new Button(dom.window.document.body, {
		label: "Save",
		title: "Save changes",
	});

	assert.deepEqual(contents, ["Save changes"]);
	assert.ok(titledButton.element.querySelector(".zeta-button-content.zeta-icon-label"));
	assert.ok(titledButton.element.querySelector(".zeta-button-label.zeta-icon-label-text"));
	assert.equal(unlabeledHoverButton.element.hasAttribute("title"), false);
	titledButton.hidden = true;
	assert.equal(titledButton.element.hidden, true);
	assert.equal(titledButton.element.classList.contains("hidden"), true);

	using labelCenteredButton = new Button(dom.window.document.body, {
		label: "Commit",
		contentAlignment: "labelCentered",
	});
	assert.equal(labelCenteredButton.element.classList.contains("label-centered"), true);

	dom.window.close();
});

function managedHover(): IManagedHover {
	return {
		visible: false,
		show() {},
		hide() {},
		update() {},
		dispose() {},
		[Symbol.dispose]() {},
	};
}
