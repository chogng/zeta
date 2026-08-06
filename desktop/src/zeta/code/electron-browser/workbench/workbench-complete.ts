import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/contrib/editor.contribution.js";
import "../../../editor/gamma/contrib/editor.contribution.js";
import { CompleteProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(CompleteProduct);
