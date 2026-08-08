import "../../../workbench/workbench.web.main.js";
import "../../../editor/gama/editor.all.js";
import { academicWorkbenchSession } from "../../../sessions/browser/academicWorkbenchSession.js";
import { academicSessionsProfile } from "../../../sessions/browser/academic/academicSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../sessions/browser/common/sessionTitlebarEntry.js";
import { AcademicProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

registerSessionsTitlebarEntry(academicSessionsProfile.titlebarActionId, "Open Academic Sessions", "../sessions/sessions-academic.html");
startBrowserWorkbench(AcademicProduct, academicWorkbenchSession);
