import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "*.integration.spec.ts",
  outputDir: "../../../output/playwright/editor-results",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://127.0.0.1:5185" },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
  ],
  webServer: process.env.ZETA_EDITOR_BROWSER_EXTERNAL_SERVER ? undefined : {
    command: "node ../../../node_modules/vite/bin/vite.js --config vite.config.ts",
    url: "http://127.0.0.1:5185/textModel.html",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  reporter: [["list"], ["html", { outputFolder: "../../../output/playwright/editor-report", open: "never" }]],
});
