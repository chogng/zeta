import type {
  ZetaRendererApi,
} from "../../app-server/common/renderer-api.js";
import type {
  INativeContextMenuApi,
} from "../../contextview/common/nativeContextMenu.js";
import type {
  INativeMenubarApi,
} from "../../menubar/common/nativeMenubar.js";

/** Capabilities exposed only by the Electron preload bridge. */
export interface ZetaElectronRendererApi extends ZetaRendererApi {
  readonly nativeContextMenu: INativeContextMenuApi;
  readonly nativeMenubar: INativeMenubarApi;
}
