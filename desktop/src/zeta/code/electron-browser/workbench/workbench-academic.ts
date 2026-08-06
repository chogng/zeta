import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/gamma/contrib/editor.contribution.js";
import {
  AcademicProduct,
} from "../../../product/common/product.js";
import {
  startElectronWorkbench,
} from "../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(AcademicProduct);
