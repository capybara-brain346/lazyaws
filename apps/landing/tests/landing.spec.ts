import { test, expect } from "@playwright/test";
import { services, site } from "../src/config";

test("service preview cycles every two seconds, wraps, and can pause", async ({
  page,
}) => {
  await page.clock.install();
  await page.clock.pauseAt(new Date());
  await page.goto("/");
  await page.getByRole("button", { name: "Pause service preview" }).click();
  await page.getByRole("button", { name: "Resume service preview" }).click();

  for (const [index, name] of services.entries()) {
    await page.clock.runFor(site.previewIntervalMs - 1);
    await expect(
      page.getByRole("button", { name, exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    await page.clock.runFor(1);
    const next = services[(index + 1) % services.length];
    await expect(
      page.getByRole("button", { name: next, exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator(".terminal-image")).toHaveAttribute(
      "src",
      `/images/demo-${next.toLowerCase()}.webp`,
    );
  }

  await page.getByRole("button", { name: "Pause service preview" }).click();
  await page.clock.runFor(site.previewIntervalMs * 2);
  await expect(
    page.getByRole("button", { name: "EC2", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "S3", exact: true }).click();
  await page.getByRole("button", { name: "Resume service preview" }).click();
  await page.clock.runFor(site.previewIntervalMs);
  await expect(
    page.getByRole("button", { name: "Lambda", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
});

test("install commands, service previews, and guide work at each viewport", async ({
  page,
  context,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/");
  expect(
    await page
      .locator('a[href]:not([href^="#"])')
      .evaluateAll((links) =>
        links.every(
          (link) =>
            link instanceof HTMLAnchorElement &&
            link.target === "_blank" &&
            link.relList.contains("noopener") &&
            link.relList.contains("noreferrer"),
        ),
      ),
  ).toBe(true);
  await expect(page.locator(".brand-mark")).toHaveCount(2);
  await expect(page.locator(".hero-foot, #tour kbd")).toHaveCount(0);
  await expect(page.locator(".section-icon, .tour-icon")).toHaveCount(5);
  await expect(page.locator(".service-tabs button img")).toHaveCount(5);
  await expect(
    page.locator(".product-preview a, .local-indicator"),
  ).toHaveCount(0);
  await expect(
    page.locator('link[rel="icon"][type="image/svg+xml"]'),
  ).toHaveAttribute("href", "/favicon.svg");
  await expect(
    page.locator('link[rel="icon"][type="image/x-icon"]'),
  ).toHaveAttribute("href", "/favicon.ico");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Your cloud.Your terminal.",
  );
  await page.getByRole("button", { name: "S3", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "S3", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByAltText(/^Real Lazy AWS S3 demo/)).toHaveJSProperty(
    "complete",
    true,
  );
  await page.getByRole("link", { name: "Get started", exact: true }).click();
  await expect(page).toHaveURL(/#install$/);
  await expect(page.getByText(site.support, { exact: true })).toBeVisible();
  await expect(page.locator("#install code").nth(0)).toHaveText(
    `curl -fsSL ${site.homepage}/install.sh | sh`,
  );
  await expect(page.locator("#install code").nth(1)).toHaveText(
    "brew install capybara-brain346/tap/lazyaws",
  );
  await page
    .getByRole("button", { name: "Copy run demo", exact: true })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "Copied to clipboard." }),
  ).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "lazyaws --demo",
  );
  await expect(page.locator(".tour-timeline > li")).toHaveCount(3);
  const productHunt = page.getByRole("link", { name: site.productHunt.alt });
  const badgeBounds = await productHunt.boundingBox();
  const headlineBounds = await page.getByRole("heading", { level: 1 }).boundingBox();
  expect(badgeBounds).not.toBeNull();
  expect(headlineBounds).not.toBeNull();
  expect(badgeBounds!.y + badgeBounds!.height).toBeLessThanOrEqual(headlineBounds!.y);
  await expect(productHunt).toHaveAttribute("href", site.productHunt.href);
  await expect(productHunt.locator("img")).toHaveAttribute(
    "src",
    site.productHunt.badge,
  );
  await expect(
    page.getByRole("link", { name: "Built by Piyush Choudhari" }),
  ).toHaveAttribute("href", "https://piyushc.com");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  for (const image of await page.locator("img").all()) {
    await image.scrollIntoViewIfNeeded();
    await expect
      .poll(() =>
        image.evaluate(
          (element) =>
            element instanceof HTMLImageElement &&
            element.complete &&
            element.naturalWidth > 0,
        ),
      )
      .toBe(true);
  }
  expect(errors).toEqual([]);
});

test("clipboard denial gives a useful recovery message", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: () => Promise.reject(new Error("denied")) },
    });
  });
  await page.goto("/#install");
  await page
    .getByRole("button", { name: "Copy run demo", exact: true })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "Copy failed." }),
  ).toBeVisible();
});
