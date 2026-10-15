import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { generatePackages } from "./generate-packages.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const generatorPath = path.join(repositoryRoot, "scripts/generate-packages.mjs");
const config = JSON.parse(await readFile(path.join(repositoryRoot, "distribution/config.json"), "utf8"));
const cargoToml = await readFile(path.join(repositoryRoot, "Cargo.toml"), "utf8");
const packageSection = cargoToml.split(/^\[/m).find((section) => section.startsWith("package]"));
const version = packageSection.match(/^version\s*=\s*"([^"]+)"\s*$/m)[1];

async function fixtureDirectory() {
  const directory = await mkdtemp(path.join(os.tmpdir(), "lazyaws-packages-"));
  const checksums = config.targets.map((target) => {
    const name = `${config.name}-v${version}-${target.triple}.tar.gz`;
    const hash = createHash("sha256").update(`test archive contents: ${name}`).digest("hex");
    return `${hash}  ${name}`;
  });

  await writeFile(path.join(directory, "SHA256SUMS"), `${checksums.join("\n")}\n`);
  return directory;
}

test("generates Homebrew and AUR metadata from all four checksums", async () => {
  const directory = await fixtureDirectory();

  try {
    const output = path.join(directory, "generated");
    const result = spawnSync(process.execPath, [generatorPath, path.join(directory, "SHA256SUMS"), output], {
      cwd: repositoryRoot,
      encoding: "utf8",
    });
    assert.equal(result.status, 0, result.stderr);

    const formula = await readFile(path.join(output, "homebrew/Formula/lazyaws.rb"), "utf8");
    const pkgbuild = await readFile(path.join(output, "aur", config.aurPackage, "PKGBUILD"), "utf8");
    const srcinfo = await readFile(path.join(output, "aur", config.aurPackage, ".SRCINFO"), "utf8");

    assert.doesNotMatch(formula, /^\s*version\s/m);
    assert.match(formula, new RegExp(`homepage "${config.homepage}"`));
    assert.doesNotMatch(formula, /^  desc "A /m);
    assert.match(formula, /on_macos do/);
    assert.match(formula, /on_linux do/);
    assert.match(formula, new RegExp(`depends_on macos: :${config.minimumMacosRelease}`));
    assert.doesNotMatch(formula, /depends_on :linux|odie/);
    assert.match(formula, /bin\.install "lazyaws"/);
    assert.match(formula, /AWS CLI v2/);
    assert.match(formula, /Demo mode works without AWS CLI or credentials/);
    assert.match(formula, new RegExp(`assert_match "lazyaws #\{version\}"`));

    for (const target of config.targets) {
      const name = `${config.name}-v${version}-${target.triple}.tar.gz`;
      const url = `${config.downloadBaseUrl}/v${version}/${name}`;
      const hash = createHash("sha256").update(`test archive contents: ${name}`).digest("hex");
      assert.ok(formula.includes(url));
      assert.ok(formula.includes(hash));
      assert.doesNotMatch(formula, /PLACEHOLDER|<checksum>/i);
    }

    assert.match(pkgbuild, /arch=\('x86_64'\)/);
    assert.match(pkgbuild, /depends=\('glibc' 'gcc-libs'\)/);
    assert.match(pkgbuild, /options=\('!strip' '!debug'\)/);
    assert.match(pkgbuild, /provides=\("lazyaws=\$pkgver"\)/);
    assert.match(pkgbuild, /conflicts=\('lazyaws'\)/);
    assert.match(pkgbuild, /\/usr\/share\/licenses\/\$pkgname\/LICENSE/);
    assert.doesNotMatch(pkgbuild, /curl|install\.sh/i);
    assert.match(srcinfo, new RegExp(`pkgname = ${config.aurPackage}`));
    assert.match(srcinfo, /options = !strip\n\toptions = !debug/);
    assert.match(srcinfo, /sha256sums = [a-f0-9]{64}/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("rejects a checksum manifest that does not cover all release archives", async () => {
  const directory = await fixtureDirectory();

  try {
    const manifestPath = path.join(directory, "invalid-SHA256SUMS");
    await writeFile(manifestPath, "not a checksum  lazyaws-v0.1.0-x86_64-unknown-linux-gnu.tar.gz\n");
    await assert.rejects(generatePackages(manifestPath, path.join(directory, "generated")), /Invalid SHA256SUMS entry/);

    const completeManifest = await readFile(path.join(directory, "SHA256SUMS"), "utf8");
    await writeFile(manifestPath, `${completeManifest.split("\n").slice(0, 3).join("\n")}\n`);
    await assert.rejects(generatePackages(manifestPath, path.join(directory, "generated")), /exactly one two-space SHA-256 entry/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
