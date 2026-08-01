import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/contrib/alphaEditor.contribution.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import { CodeProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(CodeProduct);
