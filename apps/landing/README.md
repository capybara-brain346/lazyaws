# Lazy AWS landing page

A one-page Next.js site with shadcn/ui buttons, Lucide icons, local fonts, generated landscape art, and real Lazy AWS demo captures.

Live site: https://lazyaws.choudhari-piyush.workers.dev

## Run

Use Node.js 20.9 or later and npm.
Workers commands require Node.js 22 or later.

```sh
cd apps/landing
npm ci
npm run dev
```

Open http://localhost:3000.

```sh
npm run lint
npm run typecheck
npm test
npm run build
npm start
```

The browser tests use Chromium at `/usr/bin/chromium`. Set `CHROMIUM_PATH` if the executable is elsewhere. The tests cover service selection, command copy, clipboard failure, required links, image loading, and page width on desktop and phone viewports.

## Continuous integration

`.github/workflows/landing.yml` runs on pushes and pull requests to `main`. It uses Node.js 22 and 24, installs from this app's lockfile with `npm ci`, caches npm downloads, then runs lint, build, type checks, and browser tests. CI installs Playwright's Chromium and its system dependencies. It does not need a system browser or Cloudflare credentials.

```text
Push or PR to main -> Node 22 / Node 24
  -> npm ci -> install Chromium -> lint -> build -> typecheck -> browser tests
```

In CI, the shared browser settings select Playwright's installed Chromium. `CHROMIUM_PATH` overrides the browser path in all environments. Local runs keep the system browser default.

```text
Before: Playwright config / demo capture -> tooling.browserPath -> system Chromium
After:  Playwright config / demo capture -> tooling.browserPath
          -> CHROMIUM_PATH, if set
          -> Playwright Chromium in CI
          -> system Chromium locally
```

## Deploy to Cloudflare Workers

The site uses a Next.js static export and Workers Static Assets. It needs no server process, database, or runtime secrets. `npm run build` creates `out/`. `npm start` serves that build in the local Workers runtime at http://localhost:3000.

```sh
cd apps/landing
npx wrangler login
npm run deploy
```

`wrangler.jsonc` sets the Worker name, Cloudflare account, compatibility date, asset directory, and 404 handling. The Worker uses the account's `workers.dev` address. Preview version URLs are disabled. Credentials stay in the local Wrangler store; do not add them to the repository.

```text
src/ + public/ --next build--> out/ --wrangler deploy--> Workers Static Assets
Workers Static Assets --HTML, CSS, JS, images--> browser
```

```text
Before: npm start -> next start -> Next.js server
After:  npm start -> wrangler dev -> out/
Added:  npm run deploy -> next build -> wrangler deploy -> Workers Static Assets
```

To test a local Workers preview, run `npm run build && npm start`, then use another terminal:

```sh
TEST_BASE_URL=http://localhost:3000 npm test
```

Set `TEST_BASE_URL` to the deployed HTTPS URL to run the same tests against production. This setting stops Playwright from starting a local Next.js server.

```text
Before: npm test -> defineConfig -> webServer -> next dev
After:  npm test -> defineConfig -> webServer -> next dev (default)
Added:  npm test -> defineConfig -> TEST_BASE_URL (external server)
```

Unknown paths return the exported `404.html` with HTTP 404. This setup supports the current static page and client controls. Server Actions, request-time server rendering, and API routes need a server adapter if added later. Deployment is manual; `npm run deploy` builds and publishes the current local source.

```text
Wrangler             Cloudflare             Browser
   |--upload assets----->|                     |
   |<--deployment URL----|                     |
   |                     |<--GET /------------|
   |                     |---HTML + assets--->|
```

## Page flow

```text
Header: logo + GitHub
          |
          v
Hero: landscape + real service previews
          |
          v
Install: clone -> demo -> AWS profile -> release build
          |
          v
Guide: matching cloud artwork + browse -> inspect -> confirm
          |
          v
Footer: logo + author link
```

The top demo preview is capped at 840px on wide screens to keep the screenshot compact. Its width is set by `--preview-width` in `src/app/base.css`. It cycles through EC2, ECS, ECR, S3, and Lambda every two seconds, then repeats. Manual selection restarts the timer. The pause control stops the cycle; resume starts it again. Set the interval in `src/config.ts` (`site.previewIntervalMs`).

The top demo preview has no full-size link or local sample data indicator. Website and guide image links open in new tabs. Section links and the skip link move within the current page.

Scrollbars are hidden. The page and command panels remain scrollable.

## Files and configuration

- `src/config.ts`: links, install commands, service names, guide content, preview interval, and copy feedback delay.
- `src/app/base.css`: shared color and type tokens. `globals.css` imports the focused base, hero, guide, and responsive styles.
- `scripts/config.mjs`: browser and capture settings.
- `next.config.ts`: Next.js settings.
- `wrangler.jsonc`: Workers deployment settings.
- `components.json`: shadcn configuration.

`page.tsx` composes the header, hero, install guide, interface guide, and footer. Only the service preview and copy buttons need client state.

```text
Browser       CopyButton        Clipboard
   | click        |                 |
   |------------->| write command   |
   |              |---------------->|
   |              | success / error |
   |              |<----------------|
   | status text  |                 |
   |<-------------|                 |
```

```text
src/config.ts --page content--> server components --HTML--> browser
Demo WebP files --selected service--> ProductPreview --image--> browser
Command text --CopyButton--> clipboard + status text
```

## Update demo images

Requirements: the Rust toolchain, tmux, ImageMagick, and Chromium. This command builds and runs Lazy AWS in demo mode. It uses local sample data and makes no AWS requests.

```sh
npm run capture:demo
```

The script captures all five services, the detail pane, and an action confirmation. It uses the Catppuccin Mocha theme. Each image has a provenance sidecar. The header and footer use the supplied sleeping-cloud artwork. Service buttons use the supplied AWS SVGs. Icon sources are flat in `assets/icons/`; the website serves copies from `public/icons/` and favicons from `public/`.

`favicon.ico` contains 16, 32, 48, 64, 128, and 256px frames. `favicon.svg` places an embedded PNG of the supplied artwork in a rounded square frame. It preserves the artwork; it is not a vector trace. `lazyaws-mark.webp` keeps the full cloud shape on a transparent background. The original `assets/icons/lazyaws.png` is kept. The unused AWS icon packs and the old drawn logo were removed.

`public/images/valley.webp` was created with ChatGPT image generation. Its exact prompt is in `scripts/hero-prompt.txt` and its adjacent JSON file. Reference screenshots stay in the repository `screenshots/` folder; they are not served by this app.

## Section artwork

The installation section and guide use five generated cloud illustrations. They share the supplied logo’s navy and gold palette. Exact prompts are in adjacent JSON files. The hero feature-label row and the guide shortcut badges are removed.

```text
assets/icons/lazyaws.png --visual reference--> generated section artwork
Section artwork --compressed WebP--> public/icons --> page sections
```

## Scope

Package install commands will be added when distribution is ready. The page currently documents source builds with Cargo. It has no sign-up, tracking, remote font requests, or backend.

The runtime dependency audit passes. The full npm audit currently reports five high-severity advisories in the development-only Next.js lint dependency chain. No compatible patched `braces` release was available when checked. Do not use these lint tools to process untrusted file patterns.
