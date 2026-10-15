# Lazy AWS landing page

A one-page Next.js site with shadcn/ui buttons, Lucide icons, local fonts, generated landscape art, and real Lazy AWS demo captures.

## Run

Use Node.js 20.9 or later and npm.

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

Package install commands will be added when distribution is ready. The page currently documents source builds with Cargo. It has no sign-up, tracking, remote font requests, or backend. It is not deployed by this change.

The runtime dependency audit passes. The full npm audit currently reports five high-severity advisories in the development-only Next.js lint dependency chain. No compatible patched `braces` release was available when checked. Do not use these lint tools to process untrusted file patterns.
