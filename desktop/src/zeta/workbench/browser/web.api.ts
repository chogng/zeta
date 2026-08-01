import type { IDisposable } from "../../base/common/lifecycle.js";
import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type {
  IAnyWorkspaceIdentifier,
} from "../../platform/workspace/common/workspace.js";

/**
 * Capabilities and identity supplied by an embedding Web application.
 *
 * The embedder owns transport authentication and must provide an API that
 * obeys the same renderer contract as the Electron preload bridge.
 */
export interface IWebWorkbenchHost {
  readonly api: IRendererHost;
  readonly workspace?: IAnyWorkspaceIdentifier;
  readonly container?: HTMLElement | null;
}

/** Inputs used to create one browser-hosted Workbench instance. */
export interface IWebWorkbenchConstructionOptions {
  readonly api: IRendererHost;
  readonly workspace?: IAnyWorkspaceIdentifier;
  readonly container: HTMLElement | null;
}

/** Lifecycle facade returned to a Web Workbench embedder. */
export interface IWebWorkbench extends IDisposable {}

declare global {
  /**
   * Optional host capabilities installed before a product entry is imported.
   */
  var zetaWebWorkbenchHost: IWebWorkbenchHost | undefined;
}
