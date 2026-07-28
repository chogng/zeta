import "../../../workbench/workbench.web.main.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import "../../../editor/prosemirror/contrib/proseMirrorEditor.contribution.js";
import {
  CompleteProduct,
} from "../../../product/common/product.js";
import {
  startWebWorkbench,
} from "../../../workbench/browser/web.factory.js";

startWebWorkbench(CompleteProduct);
