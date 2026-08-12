import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/editor.academic.all.js";
import "../../../workbench/contrib/academic/browser/academicEditor.contribution.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";
import { academicWorkbenchSession } from "../../browser/workbench/academicWorkbenchSession.js";

await startElectronWorkbench(AcademicProduct, academicWorkbenchSession);
