import { defineConfig, devices } from "@playwright/test";
import { tooling } from "./scripts/config.mjs";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  use: {
    baseURL: tooling.testBaseUrl || tooling.url,
    launchOptions: {
      executablePath: tooling.browserPath,
      args: tooling.browserArgs,
    },
  },
  projects: [
    {
      name: "desktop",
      use: { ...devices["Desktop Chrome"], viewport: tooling.desktop },
    },
    {
      name: "mobile",
      use: { ...devices["iPhone 13"], defaultBrowserType: "chromium" },
    },
  ],
  webServer: tooling.testBaseUrl
    ? undefined
    : {
        command: "npm run dev -- --port 3000",
        url: tooling.url,
        reuseExistingServer: !process.env.CI,
      },
});
