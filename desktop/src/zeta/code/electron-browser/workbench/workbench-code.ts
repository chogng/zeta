import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/editor.all.js";
import { codeWorkbenchSession } from "../../../sessions/browser/codeWorkbenchSession.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(ZetaDesktopProduct, codeWorkbenchSession);
