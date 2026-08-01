import "../../../workbench/workbench.web.main.js";
import "../../../editor/alpha/contrib/alphaEditor.contribution.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import "../../../editor/prosemirror/contrib/proseMirrorEditor.contribution.js";
import { CompleteProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(CompleteProduct);
