import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  // WASM hydration: the form is interactive once the button exists.
  await expect(page.getByRole("button", { name: "Evaluate" })).toBeVisible();
});

test("local mode evaluates the demo catalogue under the example policy", async ({
  page,
}) => {
  // Defaults: demo snapshot + "no US/CN data flow, self-host OK" policy,
  // code_generation / public_content / c0.
  await page.getByRole("button", { name: "Evaluate" }).click();

  const table = page.getByRole("table");
  await expect(table).toBeVisible();
  // Self-hostable models survive; API-only US models are out of the list.
  await expect(table).toContainText("mistralai/mistral-large");
  await expect(table).toContainText("meta/llama-3-3-70b");
  await expect(table).not.toContainText("openai/gpt-4o");
  await expect(page.getByText(/ineligible: [1-9]/)).toBeVisible();
});

test("explain shows a rule-by-rule verdict", async ({ page }) => {
  await page.getByRole("button", { name: "Evaluate" }).click();
  await page.getByRole("button", { name: "explain" }).first().click();

  await expect(page.getByRole("heading", { name: "Verdict" })).toBeVisible();
  await expect(page.getByText("ELIGIBLE").first()).toBeVisible();
  await expect(page.getByText(/self-hosted/).first()).toBeVisible();
});

test("a sensitive PII need tightens verdicts (fail-closed, monotonic)", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Evaluate" }).click();
  const relaxed = await page
    .getByRole("table")
    .locator("tbody tr")
    .count();

  await page.getByLabel("purpose").selectOption("personal_data");
  await page.getByLabel("sensitivity").selectOption("c2");
  await page.getByRole("button", { name: "Evaluate" }).click();
  const strict = await page.getByRole("table").locator("tbody tr").count();

  expect(strict).toBeLessThanOrEqual(relaxed);
});

test("server mode rejects an empty API URL before any request", async ({ page }) => {
  await page
    .getByLabel("Server mode (query your org's clearance-api instead)")
    .check();
  await page.getByRole("button", { name: "Evaluate" }).click();

  await expect(page.getByRole("alert")).toContainText(
    "API base URL is required in server mode",
  );
  await expect(page.getByRole("table")).toHaveCount(0);

  // The failure must carry the design system's error tone. Without it the
  // alert keeps the default accent bar — the same green the system uses for
  // non-error callouts — so a failure would read as a neutral notice.
  const alert = page.getByRole("alert");
  await expect(alert).toHaveAttribute("data-tone", "error");
  await expect(alert).toHaveCSS(
    "border-left-color",
    await alert.evaluate((node) =>
      getComputedStyle(node).getPropertyValue("color").trim(),
    ),
  );
});

test("attribution and no-redistribution notice are always visible", async ({
  page,
}) => {
  await expect(
    page.getByRole("link", { name: "Artificial Analysis" }),
  ).toBeVisible();
  await expect(page.getByText(/internal-use-only/)).toBeVisible();
});
