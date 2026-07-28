import "../../../workbench/browser/workbench.contribution.js";
import "../../../workbench/electron-browser/desktop.contribution.js";
import "../../../editor/monaco/contrib/monacoEditor.contribution.js";
import {
  CodeProduct,
} from "../../../product/common/product.js";
import {
  startElectronWorkbench,
} from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(CodeProduct);
