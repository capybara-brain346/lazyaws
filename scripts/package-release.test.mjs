import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = new URL("..", import.meta.url);
const rootPath = fileURLToPath(root);
const config = JSON.parse(readFileSync(new URL("../distribution/config.json", import.meta.url), "utf8"));
const manifest = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
const version = manifest.match(/^version\s*=\s*"(\d+\.\d+\.\d+)"$/m)[1];
const script = fileURLToPath(new URL("./package-release.mjs", import.meta.url));

test("packages only the release files and writes matching SHA-256 lines", () => {
  const temp = mkdtempSync(join(tmpdir(), "lazyaws-package-test-"));

  try {
    const binary = join(temp, "lazyaws-test-binary");
    const output = join(temp, "dist");
    writeFileSync(binary, "test binary\n");
    chmodSync(binary, 0o755);

    const triple = config.targets[0].triple;
    const filename = `lazyaws-v${version}-${triple}.tar.gz`;
    const run = spawnSync(process.execPath, [script, triple, binary, output], {
      cwd: rootPath,
      encoding: "utf8",
    });
    assert.equal(run.status, 0, run.stderr);

    const archive = join(output, filename);
    const entries = execFileSync("tar", ["-tzf", archive], { encoding: "utf8" }).trim().split("\n");
    assert.deepEqual(entries, ["lazyaws", "LICENSE", "README.txt"]);

    for (const target of config.targets.slice(1)) {
      writeFileSync(join(output, `lazyaws-v${version}-${target.triple}.tar.gz`), "other archive");
    }
    execFileSync(process.execPath, [script, "--checksums", output], { cwd: rootPath });

    const checksums = readFileSync(join(output, "SHA256SUMS"), "utf8");
    const lines = checksums.trimEnd().split("\n");
    assert.equal(lines.length, config.targets.length);
    for (const line of lines) assert.match(line, /^[a-f0-9]{64}  lazyaws-v\d+\.\d+\.\d+-[a-z0-9_-]+\.tar\.gz$/);
    const actual = createHash("sha256").update(readFileSync(archive)).digest("hex");
    assert.equal(lines.find((line) => line.endsWith(filename)).split("  ")[0], actual);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
});
