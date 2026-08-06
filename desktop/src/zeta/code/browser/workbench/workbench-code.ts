import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/editor.all.js";
import { codeWorkbenchSession } from "../../../sessions/browser/codeWorkbenchSession.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(ZetaDesktopProduct, codeWorkbenchSession);
