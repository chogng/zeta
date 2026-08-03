import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./test/smoke",
  outputDir: "./output/playwright/test-results",
  fullyParallel: false,
  workers: 1,
  projects: [
    { name: "ui" },
    { name: "desktop" },
  ],
  timeout: 45_000,
  expect: {
    timeout: 10_000,
  },
  reporter: [
    ["list"],
    ["html", { outputFolder: "./output/playwright/report", open: "never" }],
  ],
});
