import "../../../workbench/workbench.web.main.js";
import "../../../editor/gama/editor.all.js";
import { academicWorkbenchSession } from "../../../sessions/browser/academicWorkbenchSession.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(AcademicProduct, academicWorkbenchSession);
