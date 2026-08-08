import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { startElectronSessions } from "../../../sessions/electron-browser/electronSessions.js";

await startElectronSessions(ZetaDesktopProduct, codeSessionsProfile);
