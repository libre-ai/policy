import { defineConfig, devices } from "@playwright/test";

// Serves the dx release build statically: the local-mode app is pure static
// files + WASM, which is exactly how the public demo deploys.
const PORT = 8321;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
  },
  webServer: {
    command: `python3 -m http.server ${PORT} -d target/dx/rumble-ai-clearance-web-app/release/web/public`,
    port: PORT,
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
