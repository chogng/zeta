import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import {
	DisposableOwner,
} from "../../../../../base/common/lifecycle.js";
import {
	type ContextMenuOptions,
	type IContextMenuService,
} from "../../../../../platform/contextview/browser/contextMenu.js";
import {
	WorkbenchContextMenuService,
} from "../../../../../workbench/services/contextmenu/browser/workbenchContextMenuService.js";

test("workbench context menu service owns and forwards its implementation", () => {
	const implementation = new TestContextMenuImplementation();
	const service = new WorkbenchContextMenuService(implementation);
	let shows = 0;
	let hides = 0;
	using showListener = service.onDidShowContextMenu(() => {
		shows += 1;
	});
	using hideListener = service.onDidHideContextMenu(() => {
		hides += 1;
	});
	const options: ContextMenuOptions = {
		anchor: { x: 10, y: 20 },
		actions: [],
	};

	service.showContextMenu(options);
	service.hideContextMenu();

	assert.equal(implementation.lastOptions, options);
	assert.equal(shows, 1);
	assert.equal(hides, 1);
	service.dispose();
	assert.equal(implementation.disposed, true);
});

class TestContextMenuImplementation
	extends DisposableOwner
	implements IContextMenuService {
	private readonly _onDidShowContextMenu = this.own(new Emitter<void>());
	private readonly _onDidHideContextMenu = this.own(new Emitter<void>());
	readonly onDidShowContextMenu = this._onDidShowContextMenu.event;
	readonly onDidHideContextMenu = this._onDidHideContextMenu.event;
	lastOptions: ContextMenuOptions | undefined;
	disposed = false;

	constructor() {
		super();
		this.defer(() => {
			this.disposed = true;
		});
	}

	showContextMenu(options: ContextMenuOptions): void {
		this.lastOptions = options;
		this._onDidShowContextMenu.fire();
	}

	hideContextMenu(): void {
		this._onDidHideContextMenu.fire();
	}
}
