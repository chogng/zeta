import type { Locator, Page } from "@playwright/test";

/** Product-level automation surface for one editor group. */
export class EditorGroup {
	readonly element: Locator;
	readonly title: Locator;
	readonly content: Locator;
	readonly watermark: Locator;
	readonly tabs: Locator;

	constructor(element: Locator) {
		this.element = element;
		this.title = element.locator(".zeta-editor-title-control");
		this.content = element.locator(".zeta-editor-group-content");
		this.watermark = this.content.locator(".zeta-editor-group-watermark");
		this.tabs = element.getByRole("tab");
	}

	async waitForReady(): Promise<void> {
		await this.element.waitFor({ state: "visible" });
		await this.title.waitFor({ state: "visible" });
		await this.content.waitFor({ state: "visible" });
	}
}

/** Product-level automation surface for the Workbench editor region. */
export class Editors {
	readonly element: Locator;
	readonly groups: Locator;

	constructor(page: Page) {
		this.element = page.locator(".zeta-workbench-editor");
		this.groups = this.element.locator(".zeta-editor-group");
	}

	groupAt(index: number): EditorGroup {
		return new EditorGroup(this.groups.nth(index));
	}

	async waitForReady(): Promise<void> {
		await this.element.waitFor({ state: "visible" });
		await this.groupAt(0).waitForReady();
	}
}
