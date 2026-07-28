import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../src/base/common/event.js";
import {
  DisposableOwner,
} from "../src/base/common/lifecycle.js";
import {
  type ContextMenuOptions,
  type IContextMenuService,
} from "../src/platform/contextview/browser/contextMenu.js";
import {
  WorkbenchContextMenuService,
} from "../src/workbench/services/contextmenu/common/contextMenuService.js";

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
  readonly #onDidShowContextMenu = this.own(new Emitter<void>());
  readonly #onDidHideContextMenu = this.own(new Emitter<void>());
  readonly onDidShowContextMenu = this.#onDidShowContextMenu.event;
  readonly onDidHideContextMenu = this.#onDidHideContextMenu.event;
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
    this.#onDidShowContextMenu.fire();
  }

  hideContextMenu(): void {
    this.#onDidHideContextMenu.fire();
  }
}
