import { AcademicProduct } from "../../../product/common/product.js";
import { academicSessionsProfile } from "../../../sessions/browser/academic/academicSessionsProfile.js";
import { startElectronSessions } from "../../../sessions/electron-browser/electronSessions.js";

await startElectronSessions(AcademicProduct, academicSessionsProfile);
