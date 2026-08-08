import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/gama/editor.all.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";
import { academicWorkbenchSession } from "../../browser/workbench/academicWorkbenchSession.js";

await startElectronWorkbench(AcademicProduct, academicWorkbenchSession);
