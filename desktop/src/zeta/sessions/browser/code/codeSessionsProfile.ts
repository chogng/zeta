import { createSessionsProfile } from "../../common/sessionsProfile.js";

/** Dedicated agent-session window for the Code product. */
export const codeSessionsProfile = createSessionsProfile({
  id: "code-sessions",
  productId: "code",
  label: "Code Sessions",
  titlebarActionId: "zeta.code.open-sessions",
  workbenchRelativePath: "../workbench/workbench-code.html",
});
