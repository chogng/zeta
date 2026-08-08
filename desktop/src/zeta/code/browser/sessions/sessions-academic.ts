import { AcademicProduct } from "../../../product/common/product.js";
import { academicSessionsProfile } from "../../../sessions/browser/academic/academicSessionsProfile.js";
import { startBrowserSessions } from "../../../sessions/browser/webSessions.js";

startBrowserSessions(AcademicProduct, academicSessionsProfile);
