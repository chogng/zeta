import "../../../browser/workbench/modes/academic.contribution.js";
import { AcademicProduct } from "../../../../product/common/product.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(AcademicProduct, defaultWorkbenchProfile);
