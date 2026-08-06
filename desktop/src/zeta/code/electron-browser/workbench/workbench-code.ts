import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/contrib/editor.contribution.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(ZetaDesktopProduct);
