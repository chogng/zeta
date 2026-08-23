import type {
	IRuntimeEnvironment,
} from "../../../base/common/environment.js";
import type {
	IBrowserViewApi,
} from "../../browser/common/browserView.js";
import type { IRendererHost } from "../../renderer/common/rendererHost.js";
import type {
	IConfigurationApi,
} from "../../configuration/common/configurationIpc.js";
import type {
	IKeybindingsResourceApi,
} from "../../keybinding/common/keybindingsResource.js";
import type { INativeKeyboardLayoutApi } from "../../keyboardLayout/common/nativeKeyboardLayout.js";
import type { IUserKeyboardLayoutApi } from "../../keyboardLayout/common/userKeyboardLayout.js";
import type {
	INativeContextMenuApi,
} from "../../../base/parts/contextmenu/common/contextmenu.js";
import type {
	INativeMenubarApi,
} from "../../menubar/common/nativeMenubar.js";
import type {
	IWorkspaceContextApi,
} from "../../workspace/common/workspaceIpc.js";
import type { IUserThemeFilesApi } from "../../theme/common/userThemeFiles.js";
import type {
	INativeHostApi,
} from "./nativeHost.js";

/** Capabilities exposed only by the Electron preload bridge. */
export interface ZetaElectronRendererApi extends IRendererHost {
	readonly environment: IRuntimeEnvironment;
	readonly browserView: IBrowserViewApi;
	readonly configuration: IConfigurationApi;
	readonly keybindings: IKeybindingsResourceApi;
	readonly keyboardLayout: INativeKeyboardLayoutApi;
	readonly userKeyboardLayout: IUserKeyboardLayoutApi;
	readonly nativeContextMenu: INativeContextMenuApi;
	readonly nativeHost: INativeHostApi;
	readonly nativeMenubar: INativeMenubarApi;
	readonly userThemes: IUserThemeFilesApi;
	readonly workspace: IWorkspaceContextApi;
}
