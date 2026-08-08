import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { startBrowserSessions } from "../../../sessions/browser/webSessions.js";

startBrowserSessions(ZetaDesktopProduct, codeSessionsProfile);
