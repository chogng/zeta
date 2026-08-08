import "./media/sessionsWorkbench.css";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type { SessionsProfile } from "../common/sessionsProfile.js";
import type { ISessionsWindowApi } from "../common/sessionsWindow.js";
import { CodeSessionsWorkbench } from "./code/codeSessionsWorkbench.js";
import { SessionsRuntime } from "./common/sessionsRuntime.js";
import { bindSessionsTheme } from "./common/sessionsTheme.js";

export interface SessionsWorkbenchOptions {
  readonly product: ProductConfiguration;
  readonly profile: SessionsProfile;
  readonly api: IRendererHost;
  readonly sessionsWindowApi?: ISessionsWindowApi;
  readonly container: HTMLElement | null;
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
    if (!container) throw new Error("Sessions renderer requires an #app container");
    this.own(bindSessionsTheme(container));
    const runtime = this.own(new SessionsRuntime(options.api));
    const sessions = this.createCodeSessions(options.profile, runtime, options.sessionsWindowApi, container.ownerDocument);
    this.element = sessions.element;
    container.replaceChildren(this.element);
    this.defer(() => this.element.remove());
  }

  private createCodeSessions(profile: SessionsProfile, runtime: SessionsRuntime, sessionsWindowApi: ISessionsWindowApi | undefined, ownerDocument: Document): CodeSessionsWorkbench {
    if (profile.id !== "code-sessions") {
      throw new TypeError(`Unsupported Code Sessions profile '${profile.id}'`);
    }
    return this.own(new CodeSessionsWorkbench(ownerDocument, profile, runtime, sessionsWindowApi));
  }
}

/** Creates one dedicated Sessions workbench from the selected product entry. */
export function startSessionsWorkbench(options: SessionsWorkbenchOptions): SessionsWorkbench {
  return new SessionsWorkbench(options);
}
