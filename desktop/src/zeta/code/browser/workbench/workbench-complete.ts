import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/contrib/editor.contribution.js";
import "../../../editor/gamma/contrib/editor.contribution.js";
import { CompleteProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(CompleteProduct);
