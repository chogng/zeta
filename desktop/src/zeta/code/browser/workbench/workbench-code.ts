import "../../../workbench/workbench.web.main.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import {
  CodeProduct,
} from "../../../product/common/product.js";
import {
  startWebWorkbench,
} from "../../../workbench/browser/web.factory.js";

startWebWorkbench(CodeProduct);
