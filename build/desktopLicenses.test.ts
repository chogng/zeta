import test from "node:test";

import { checkDesktopLicenseCopies } from "./desktop/resources/syncDesktopLicenses.ts";

test("Desktop release license copies match their component owners", async () => {
  await checkDesktopLicenseCopies();
});
