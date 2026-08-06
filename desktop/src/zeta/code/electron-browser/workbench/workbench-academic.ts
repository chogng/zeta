import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/gama/editor.all.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";
import { academicWorkbenchSession } from "../../../sessions/browser/academicWorkbenchSession.js";

await startElectronWorkbench(AcademicProduct, academicWorkbenchSession);
