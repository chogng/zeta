import "../../../workbench/workbench.web.main.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import "../../../editor/prosemirror/contrib/proseMirrorEditor.contribution.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import {
  CompleteProduct,
} from "../../../product/common/product.js";
import type {
  ZetaRendererApi,
} from "../../../platform/app-server/common/renderer-api.js";
import {
  startBrowserWorkbench as startProductWorkbench,
} from "../../../workbench/browser/browserWorkbench.js";

/** Starts the browser-hosted Zeta Complete workbench. */
export function startBrowserWorkbench(
  api: ZetaRendererApi,
  container: HTMLElement | null,
): IDisposable {
  return startProductWorkbench(
    CompleteProduct,
    api,
    container,
  );
}
