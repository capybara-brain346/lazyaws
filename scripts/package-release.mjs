import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const config = JSON.parse(readFileSync(join(root, "distribution/config.json"), "utf8"));

function packageVersion() {
  let inPackage = false;

  for (const line of readFileSync(join(root, "Cargo.toml"), "utf8").split(/\r?\n/)) {
    if (/^\s*\[/.test(line)) {
      if (inPackage) break;
      inPackage = line.trim() === "[package]";
    }

    if (inPackage) {
      const match = line.match(/^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$/);
      if (match) return match[1];
    }
  }

  throw new Error("Cargo.toml must define a stable package version in [package]");
}

const version = packageVersion();
const expectedArchives = config.targets.map(({ triple }) => `lazyaws-v${version}-${triple}.tar.gz`);

function metadata() {
  if (!Array.isArray(config.targets) || config.targets.length === 0) {
    throw new Error("distribution/config.json must define release targets");
  }

  console.log(JSON.stringify({
    version,
    targets: config.targets.map(({ triple, runner }) => ({ triple, runner })),
  }));
}

function packageArchive(triple, binaryArgument, outputArgument) {
  if (!config.targets.some((target) => target.triple === triple)) {
    throw new Error(`unknown release target: ${triple}`);
  }

  const binary = resolve(binaryArgument);
  if (!existsSync(binary)) throw new Error(`release binary does not exist: ${binary}`);

  const output = resolve(outputArgument);
  mkdirSync(output, { recursive: true });

  const staging = mkdtempSync(join(tmpdir(), "lazyaws-release-"));
  const archiveName = `lazyaws-v${version}-${triple}.tar.gz`;
  const archivePath = join(output, archiveName);

  try {
    copyFileSync(binary, join(staging, "lazyaws"));
    chmodSync(join(staging, "lazyaws"), 0o755);
    copyFileSync(join(root, "LICENSE"), join(staging, "LICENSE"));
    writeFileSync(
      join(staging, "README.txt"),
      `lazyaws ${version}\nInstall: move ./lazyaws to a directory on your PATH.\nRun ./lazyaws --help for command options.\nAWS features require AWS CLI v2 and valid AWS credentials.\n${config.homepage}\n`,
    );

    execFileSync("tar", ["-czf", archivePath, "-C", staging, "lazyaws", "LICENSE", "README.txt"], {
      stdio: "inherit",
    });
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }

  console.log(archivePath);
}

function writeChecksums(directoryArgument) {
  const directory = resolve(directoryArgument);
  const archives = readdirSync(directory)
    .filter((name) => name.endsWith(".tar.gz"))
    .sort();

  if (JSON.stringify(archives) !== JSON.stringify([...expectedArchives].sort())) {
    throw new Error(`expected release archives: ${expectedArchives.join(", ")}`);
  }

  const lines = archives.map((name) => {
    const digest = createHash("sha256").update(readFileSync(join(directory, name))).digest("hex");
    return `${digest}  ${basename(name)}`;
  });

  writeFileSync(join(directory, "SHA256SUMS"), `${lines.join("\n")}\n`);
}

const args = process.argv.slice(2);
if (args[0] === "--metadata" && args.length === 1) {
  metadata();
} else if (args[0] === "--checksums" && args.length === 2) {
  writeChecksums(args[1]);
} else if (args.length === 3) {
  packageArchive(...args);
} else {
  throw new Error("usage: package-release.mjs <target-triple> <binary> <output-dir> | --metadata | --checksums <dir>");
}
