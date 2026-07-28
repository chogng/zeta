import type {
  IRuntimeEnvironment,
} from "../../../base/common/environment.js";
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
} from "../../contextview/common/nativeContextMenu.js";
import type {
  INativeMenubarApi,
} from "../../menubar/common/nativeMenubar.js";
import type {
  IWorkspaceContextApi,
} from "../../workspace/common/workspaceIpc.js";

/** Capabilities exposed only by the Electron preload bridge. */
export interface ZetaElectronRendererApi extends ZetaRendererApi {
  readonly environment: IRuntimeEnvironment;
  readonly configuration: IConfigurationApi;
  readonly keybindings: IKeybindingsResourceApi;
  readonly nativeContextMenu: INativeContextMenuApi;
  readonly nativeMenubar: INativeMenubarApi;
  readonly workspace: IWorkspaceContextApi;
}
