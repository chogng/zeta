import "./media/sessionsWorkbench.css";
import "./actions/sessionsChatActions.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import type { IConfigurationApi } from "../../platform/configuration/common/configurationIpc.js";
import type { IKeybindingsResourceApi } from "../../platform/keybinding/common/keybindingsResource.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type { IWorkspaceContextApi } from "../../platform/workspace/common/workspaceIpc.js";
import { IStorageService } from "../../platform/storage/common/storage.js";
import type { WorkbenchContextMenuServiceFactory } from "../../workbench/services/contextmenu/browser/workbenchContextMenuService.js";
import { BrowserStorageService } from "../../workbench/services/storage/browser/storageService.js";
import type { SessionsProfile } from "../common/sessionsProfile.js";
import type { ISessionsWindowApi } from "../common/sessionsWindow.js";
import { CodeSessionsWorkbench, type CodeSessionsWorkbenchOptions } from "./code/codeSessionsWorkbench.js";
import { SessionsRuntime } from "./common/sessionsRuntime.js";
import { bindSessionsTheme } from "./common/sessionsTheme.js";

export interface SessionsWorkbenchOptions {
  readonly product: ProductConfiguration;
  readonly profile: SessionsProfile;
  readonly api: IRendererHost;
  readonly sessionsWindowApi?: ISessionsWindowApi;
  readonly workspaceApi?: IWorkspaceContextApi;
  readonly configurationApi?: IConfigurationApi;
  readonly keybindingsResourceApi?: IKeybindingsResourceApi;
  readonly createContextMenuService: WorkbenchContextMenuServiceFactory;
  readonly container: HTMLElement;
}

/** Standalone product Sessions host that intentionally does not construct WorkbenchLayout. */
export class SessionsWorkbench extends DisposableOwner {
  readonly element: HTMLElement;

  constructor(options: SessionsWorkbenchOptions) {
    super();
    if (options.profile.productId !== options.product.id) {
      throw new TypeError(`Sessions profile '${options.profile.id}' belongs to '${options.profile.productId}', not '${options.product.id}'`);
    }
    const container = options.container;
    const ownerWindow = container.ownerDocument.defaultView;
    if (!ownerWindow) throw new Error("Sessions renderer requires an owner window");
    this.own(bindSessionsTheme(container));
    const runtime = this.own(new SessionsRuntime(options.api, {
      ...(options.sessionsWindowApi ? { sessionsWindowApi: options.sessionsWindowApi } : {}),
      ...(options.workspaceApi ? { workspaceApi: options.workspaceApi } : {}),
    }));
    const storage = this.own(new BrowserStorageService({
      ownerWindow,
      applicationId: options.product.storageNamespace,
      workspaceId: "sessions",
      profileId: options.profile.id,
    }));
    runtime.services.set(IStorageService, storage);
    const sessions = this.createCodeSessions(container, {
      profile: options.profile,
      runtime,
      sessionsWindowApi: options.sessionsWindowApi,
      configurationApi: options.configurationApi,
      keybindingsResourceApi: options.keybindingsResourceApi,
      createContextMenuService: options.createContextMenuService,
      storageService: storage,
    });
    this.element = sessions.element;
    container.replaceChildren(this.element);
    sessions.layout();
    this.defer(() => this.element.remove());
  }

  private createCodeSessions(container: HTMLElement, options: CodeSessionsWorkbenchOptions): CodeSessionsWorkbench {
    if (options.profile.id !== "code-sessions") {
      throw new TypeError(`Unsupported Code Sessions profile '${options.profile.id}'`);
    }
    return this.own(new CodeSessionsWorkbench(container, options));
  }
}

/** Creates one dedicated Sessions workbench from the selected product entry. */
export function startSessionsWorkbench(options: SessionsWorkbenchOptions): SessionsWorkbench {
  return new SessionsWorkbench(options);
}
