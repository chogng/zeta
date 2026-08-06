import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/editor.all.js";
import "../../../editor/gama/editor.all.js";
import { completeWorkbenchSession } from "../../../sessions/browser/completeWorkbenchSession.js";
import { CompleteProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(CompleteProduct, completeWorkbenchSession);
