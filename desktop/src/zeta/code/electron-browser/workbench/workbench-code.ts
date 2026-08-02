import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/contrib/alphaEditor.contribution.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(ZetaDesktopProduct);
