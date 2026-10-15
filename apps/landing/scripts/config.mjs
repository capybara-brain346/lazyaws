export const tooling = {
  browserPath:
    process.env.CHROMIUM_PATH ||
    (process.env.CI ? undefined : "/usr/bin/chromium"),
  browserArgs: ["--no-sandbox"],
  url: "http://localhost:3000",
  testBaseUrl: process.env.TEST_BASE_URL,
  desktop: { width: 1440, height: 1000 },
  mobile: { width: 390, height: 844 },
  demo: {
    columns: 120,
    rows: 26,
    fontSize: 14,
    lineHeight: 1.18,
    keyDelayMs: 100,
    renderDelayMs: 200,
    startupDelayMs: 500,
    quality: "93",
    background: "#1e1e2e",
    foreground: "#cdd6f4",
  },
};
