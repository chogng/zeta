import "./academic.contribution.js";
import { AcademicProduct } from "../../../../product/common/product.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startBrowserWorkbench } from "../../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(AcademicProduct, defaultWorkbenchProfile);
