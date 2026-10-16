import { chmod, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const outputPath = resolve(root, "apps/landing/public/install.sh");

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function shellValue(config, key) {
  const value = config[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`distribution/config.json must define ${key}`);
  }
  return shellQuote(value);
}

export async function renderInstaller() {
  const [configText, cargoText, template] = await Promise.all([
    readFile(resolve(root, "distribution/config.json"), "utf8"),
    readFile(resolve(root, "Cargo.toml"), "utf8"),
    readFile(resolve(root, "distribution/install.sh.in"), "utf8"),
  ]);
  const config = JSON.parse(configText);
  const packageSection = cargoText.split(/^\[package\]\s*$/m)[1]?.split(/^\[/m)[0] ?? "";
  const packageName = packageSection.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
  const version = packageSection.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

  if (!packageName || packageName !== config.name || !version) {
    throw new Error("Cargo.toml package name or version does not match installer config");
  }
  if (!/^[A-Za-z0-9_-]+$/.test(config.name)) {
    throw new Error("distribution/config.json name must be a safe executable name");
  }
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`Cargo.toml version must be stable X.Y.Z: ${version}`);
  }
  for (const key of ["latestUrl", "downloadBaseUrl"]) {
    if (!/^https:\/\/[^\s]+$/.test(config[key] ?? "")) {
      throw new Error(`distribution/config.json ${key} must use HTTPS`);
    }
  }
  if (config.downloadBaseUrl.endsWith("/")) {
    throw new Error("distribution/config.json downloadBaseUrl must not end with a slash");
  }
  for (const key of ["connectTimeoutSeconds", "downloadTimeoutSeconds"]) {
    if (!Number.isInteger(config[key]) || config[key] < 1) {
      throw new Error(`distribution/config.json ${key} must be a positive integer`);
    }
  }
  for (const key of ["minimumUbuntuVersion", "minimumMacosVersion"]) {
    if (!/^\d+\.\d+$/.test(config[key] ?? "")) {
      throw new Error(`distribution/config.json ${key} must use X.Y format`);
    }
  }

  if (!Array.isArray(config.targets) || config.targets.length === 0) {
    throw new Error("distribution/config.json must define release targets");
  }
  const targetKeys = new Set();
  const targetCases = config.targets.map(({ triple, os, arch }) => {
    if (!["darwin", "linux"].includes(os) || !arch || !/^[A-Za-z0-9_-]+$/.test(triple ?? "")) {
      throw new Error("distribution/config.json contains an invalid target");
    }
    const key = `${os}:${arch}`;
    if (targetKeys.has(key)) throw new Error(`Duplicate installer target: ${key}`);
    targetKeys.add(key);
    return `  ${shellQuote(key)}) target=${shellQuote(triple)} ;;`;
  }).join("\n");

  const replacements = {
    "@@PRODUCT_NAME@@": shellValue(config, "name"),
    "@@INSTALL_DIR@@": shellValue(config, "installDir"),
    "@@LATEST_URL@@": shellValue(config, "latestUrl"),
    "@@DOWNLOAD_BASE_URL@@": shellValue(config, "downloadBaseUrl"),
    "@@CONNECT_TIMEOUT@@": shellQuote(config.connectTimeoutSeconds),
    "@@DOWNLOAD_TIMEOUT@@": shellQuote(config.downloadTimeoutSeconds),
    "@@MINIMUM_UBUNTU_VERSION@@": shellValue(config, "minimumUbuntuVersion"),
    "@@MINIMUM_MACOS_VERSION@@": shellValue(config, "minimumMacosVersion"),
    "@@TARGET_CASES@@": targetCases,
  };

  let generated = template;
  for (const [placeholder, value] of Object.entries(replacements)) {
    generated = generated.replaceAll(placeholder, () => value);
  }
  if (generated.includes("@@")) throw new Error("Installer template has an unknown placeholder");
  return generated;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const generated = await renderInstaller();
  await writeFile(outputPath, generated, "utf8");
  await chmod(outputPath, 0o755);
  process.stdout.write(`Generated ${outputPath}\n`);
}
