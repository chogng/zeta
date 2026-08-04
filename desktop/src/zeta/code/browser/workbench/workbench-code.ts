import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/contrib/editor.contribution.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(ZetaDesktopProduct);
