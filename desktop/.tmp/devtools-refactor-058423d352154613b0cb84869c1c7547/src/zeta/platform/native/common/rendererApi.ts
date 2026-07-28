import type {
  IRuntimeEnvironment,
} from "../../../base/common/environment.js";
import type {
  IBrowserViewApi,
} from "../../browser/common/browserView.js";
import type {
  ZetaRendererApi,
} from "../../app-server/common/renderer-api.js";
import type {
  IConfigurationApi,
} from "../../configuration/common/configuration.js";
import type {
  IKeybindingsResourceApi,
} from "../../keybinding/common/keybindingsResource.js";
import type {
  INativeContextMenuApi,
} from "../../../base/parts/contextmenu/common/contextmenu.js";
import type {
  INativeMenubarApi,
} from "../../menubar/common/nativeMenubar.js";
import type {
  IWorkspaceContextApi,
} from "../../workspace/common/workspaceIpc.js";
import type {
  INativeHostApi,
} from "./nativeHost.js";

/** Capabilities exposed only by the Electron preload bridge. */
export interface ZetaElectronRendererApi extends ZetaRendererApi {
  readonly environment: IRuntimeEnvironment;
  readonly browserView: IBrowserViewApi;
  readonly configuration: IConfigurationApi;
  readonly keybindings: IKeybindingsResourceApi;
  readonly nativeContextMenu: INativeContextMenuApi;
  readonly nativeHost: INativeHostApi;
  readonly nativeMenubar: INativeMenubarApi;
  readonly workspace: IWorkspaceContextApi;
}
