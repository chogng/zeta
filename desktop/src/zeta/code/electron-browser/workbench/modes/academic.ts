import "../../../browser/workbench/modes/academic.contribution.js";
import { AcademicProduct } from "../../../../product/common/product.js";
import { defaultWorkbenchSession } from "../../../../workbench/browser/defaultWorkbenchSession.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(AcademicProduct, defaultWorkbenchSession);
