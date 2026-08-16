import "./academic.contribution.js";
import { AcademicProduct } from "../../../../product/common/product.js";
import { defaultWorkbenchSession } from "../../../../workbench/browser/defaultWorkbenchSession.js";
import { startBrowserWorkbench } from "../../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(AcademicProduct, defaultWorkbenchSession);
