import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/editor.all.js";
import "../../../editor/gama/editor.all.js";
import { completeWorkbenchSession } from "../../../sessions/browser/completeWorkbenchSession.js";
import { CompleteProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(CompleteProduct, completeWorkbenchSession);
