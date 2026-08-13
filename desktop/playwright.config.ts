import { defineConfig } from "@playwright/test";

const browserServerMode = process.env.ZETA_PLAYWRIGHT_SERVER;
const browserProjects = browserServerMode === "disconnected"
  ? [{ name: "browser-ui", use: { baseURL: "http://127.0.0.1:5173" } }]
  : browserServerMode === "full"
    ? [{ name: "browser-app-server", use: { baseURL: "http://127.0.0.1:5174" } }]
    : [];

export default defineConfig({
  testDir: "./test/smoke",
  outputDir: "./output/playwright/test-results",
  fullyParallel: false,
  workers: 1,
  projects: [
    ...browserProjects,
    { name: "electron-ui" },
    { name: "electron-academic-ui", testMatch: "**/areas/academic/academic-workbench.spec.ts" },
    { name: "electron-app-server" },
    { name: "electron-editor-code-app-server", testMatch: "**/areas/editor/editor-open.spec.ts" },
    { name: "electron-pdf-corpus-code-app-server", testMatch: "**/areas/pdf/pdf-academic-corpus.spec.ts" },
    { name: "electron-editor-academic-app-server", testMatch: "**/areas/editor/academic-open.spec.ts" },
  ],
  webServer: process.env.ZETA_SMOKE_BROWSER_EXTERNAL_SERVER
    ? undefined
    : browserServerMode === "disconnected"
    ? {
        command: "corepack pnpm run dev:web",
        url: "http://127.0.0.1:5173/",
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      }
    : browserServerMode === "full"
      ? {
          command: "corepack pnpm run dev:web:full",
          url: "http://127.0.0.1:5174/",
          reuseExistingServer: !process.env.CI,
          timeout: 120_000,
        }
      : undefined,
  timeout: 45_000,
  expect: {
    timeout: 10_000,
  },
  reporter: [
    ["list"],
    ["html", { outputFolder: "./output/playwright/report", open: "never" }],
  ],
});
