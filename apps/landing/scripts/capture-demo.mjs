import { execFileSync } from "node:child_process";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";
import { tooling } from "./config.mjs";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const session = `lazyaws-site-${process.pid}`;
const target = session;
const tmux = (...args) => execFileSync("tmux", args, { encoding: "utf8" });
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const out = fileURLToPath(new URL("../public/images", import.meta.url));
const browser = await chromium.launch({
  executablePath: tooling.browserPath,
  args: tooling.browserArgs,
});

try {
  execFileSync("cargo", ["build"], { cwd: root, stdio: "inherit" });
  mkdirSync(out, { recursive: true });
  tmux(
    "new-session",
    "-d",
    "-s",
    session,
    "-x",
    String(tooling.demo.columns),
    "-y",
    String(tooling.demo.rows),
    "-c",
    root,
    "env -u NO_COLOR TERM=xterm-256color COLORTERM=truecolor ./target/debug/lazyaws --demo",
  );
  await pause(tooling.demo.startupDelayMs);
  for (const key of ["t", "j", "Enter"]) {
    tmux("send-keys", "-t", target, key);
    await pause(tooling.demo.keyDelayMs);
  }
  await pause(tooling.demo.renderDelayMs);
  const page = await browser.newPage({
    viewport: tooling.desktop,
    deviceScaleFactor: 1,
  });
  await page.setContent(
    '<style>body{margin:0;background:#1e1e2e}#terminal{width:max-content;padding:16px}.xterm-viewport{overflow:hidden!important}</style><div id="terminal"></div>',
  );
  await page.addStyleTag({
    path: resolve("node_modules/@xterm/xterm/css/xterm.css"),
  });
  await page.addScriptTag({
    path: resolve("node_modules/@xterm/xterm/lib/xterm.js"),
  });
  const font = readFileSync(
    resolve(
      "node_modules/@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2",
    ),
  ).toString("base64");
  await page.addStyleTag({
    content: `@font-face{font-family:DemoMono;src:url(data:font/woff2;base64,${font})}`,
  });
  await page.evaluate(async (config) => {
    await document.fonts.load(`${config.fontSize}px DemoMono`);
    window.term = new window.Terminal({
      cols: config.columns,
      rows: config.rows,
      fontFamily: "DemoMono",
      fontSize: config.fontSize,
      lineHeight: config.lineHeight,
      cursorBlink: false,
      theme: { background: config.background, foreground: config.foreground },
    });
    window.term.open(document.getElementById("terminal"));
  }, tooling.demo);
  async function capture(name) {
    await pause(tooling.demo.renderDelayMs);
    const ansi = tmux("capture-pane", "-p", "-e", "-t", target);
    await page.evaluate(
      (text) =>
        new Promise((done) => {
          window.term.reset();
          window.term.write(
            text.replaceAll("\n", "\r\n").trimEnd() + "\x1b[?25l",
            done,
          );
        }),
      ansi,
    );
    await page.locator("#terminal").screenshot({ path: `${out}/${name}.png` });
    execFileSync("magick", [
      `${out}/${name}.png`,
      "-quality",
      tooling.demo.quality,
      `${out}/${name}.webp`,
    ]);
    execFileSync("rm", [`${out}/${name}.png`]);
    writeFileSync(
      `${out}/${name}.webp.json`,
      JSON.stringify(
        {
          prompt: `Source: actual Lazy AWS --demo session, ${name}, Catppuccin Mocha theme. Captured by scripts/capture-demo.mjs through tmux and xterm.js. Local sample resources only.`,
          source: "repository demo mode",
        },
        null,
        2,
      ) + "\n",
    );
  }
  await capture("demo-ec2");
  for (const service of ["ecs", "ecr", "s3", "lambda"]) {
    tmux("send-keys", "-t", target, "]");
    await capture(`demo-${service}`);
  }
  tmux("send-keys", "-t", target, "[", "[", "[", "[", "2", "Enter");
  await capture("demo-details");
  tmux("send-keys", "-t", target, "2", "a", "Enter");
  await capture("demo-action");
} finally {
  try {
    tmux("kill-session", "-t", session);
  } catch {}
  await browser.close();
}
