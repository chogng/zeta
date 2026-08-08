import { createSessionsProfile } from "../../common/sessionsProfile.js";

/** Dedicated research-session window for the Academic product. */
export const academicSessionsProfile = createSessionsProfile({
  id: "academic-sessions",
  productId: "academic",
  label: "Academic Sessions",
  titlebarActionId: "zeta.academic.open-sessions",
  workbenchRelativePath: "../workbench/workbench-academic.html",
});
