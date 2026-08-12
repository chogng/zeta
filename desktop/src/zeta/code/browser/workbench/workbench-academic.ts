import "../../../workbench/workbench.web.main.js";
import "../../../editor/editor.academic.all.js";
import "../../../workbench/contrib/academic/browser/academicEditor.contribution.js";
import { academicWorkbenchSession } from "./academicWorkbenchSession.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(AcademicProduct, academicWorkbenchSession);
