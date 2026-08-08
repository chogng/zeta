import "./media/sessionsWorkbench.css";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import type { IBrowserViewApi } from "../../platform/browser/common/browserView.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type { SessionsProfile } from "../common/sessionsProfile.js";
import { AcademicSessionsWorkbench } from "./academic/academicSessionsWorkbench.js";
import { CodeSessionsWorkbench } from "./code/codeSessionsWorkbench.js";
import { SessionsRuntime } from "./common/sessionsRuntime.js";

export interface SessionsWorkbenchOptions {
  readonly product: ProductConfiguration;
  readonly profile: SessionsProfile;
  readonly api: IRendererHost;
  readonly browserViewApi?: IBrowserViewApi;
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
    const runtime = this.own(new SessionsRuntime(options.api));
    const sessions = this.createProductSessions(options.profile, runtime, options.browserViewApi, container.ownerDocument);
    this.element = sessions.element;
    container.replaceChildren(this.element);
    this.defer(() => this.element.remove());
  }

  private createProductSessions(profile: SessionsProfile, runtime: SessionsRuntime, browserViewApi: IBrowserViewApi | undefined, ownerDocument: Document): CodeSessionsWorkbench | AcademicSessionsWorkbench {
    switch (profile.id) {
      case "code-sessions":
        return this.own(new CodeSessionsWorkbench(ownerDocument, profile, runtime));
      case "academic-sessions":
        return this.own(new AcademicSessionsWorkbench(ownerDocument, profile, runtime, browserViewApi));
      default:
        throw new TypeError(`Unsupported Sessions profile '${profile.id}'`);
    }
  }
}

/** Creates one dedicated Sessions workbench from the selected product entry. */
export function startSessionsWorkbench(options: SessionsWorkbenchOptions): SessionsWorkbench {
  return new SessionsWorkbench(options);
}
