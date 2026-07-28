import "../../../workbench/browser/workbench.contribution.js";
import "../../../workbench/electron-browser/desktop.contribution.js";
import "../../../editor/prosemirror/contrib/proseMirrorEditor.contribution.js";
import {
  AcademicProduct,
} from "../../../product/common/product.js";
import {
  startElectronWorkbench,
} from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(AcademicProduct);
