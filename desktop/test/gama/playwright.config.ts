import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "gama.integration.spec.ts",
  outputDir: "../../output/playwright/gama-results",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://127.0.0.1:5186" },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
  ],
  webServer: {
    command: "pnpm exec vite --config vite.config.ts",
    url: "http://127.0.0.1:5186/gama.html",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  reporter: [["list"], ["html", { outputFolder: "../../output/playwright/gama-report", open: "never" }]],
});
