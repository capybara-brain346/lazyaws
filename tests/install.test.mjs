import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { accessSync, constants, existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { delimiter, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { renderInstaller } from "../scripts/generate-installer.mjs";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const installerPath = join(root, "apps/landing/public/install.sh");
const config = JSON.parse(await readFile(join(root, "distribution/config.json"), "utf8"));
const cargo = await readFile(join(root, "Cargo.toml"), "utf8");
const version = `v${cargo.match(/^version\s*=\s*"([^"]+)"/m)[1]}`;
const archive = `${config.name}-${version}-x86_64-unknown-linux-gnu.tar.gz`;

function executableInPath(name) {
  for (const directory of process.env.PATH.split(delimiter)) {
    const candidate = join(directory, name);
    try {
      accessSync(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Continue through PATH.
    }
  }
  throw new Error(`${name} is not available in PATH`);
}

async function fixture({ corruptChecksum = false, latestTagUrl = `${config.latestUrl.replace(/\/latest$/, "")}/tag/${version}` } = {}) {
  const directory = await mkdtemp(join(tmpdir(), "lazyaws installer test "));
  const binDirectory = join(directory, "mock bin");
  const fixtureDirectory = join(directory, "downloads");
  const archiveDirectory = join(directory, "archive contents");
  const homeDirectory = join(directory, "home with spaces");
  const curlLog = join(directory, "curl requests.txt");
  await Promise.all([
    mkdir(binDirectory, { recursive: true }),
    mkdir(fixtureDirectory),
    mkdir(archiveDirectory),
    mkdir(homeDirectory),
  ]);

  await writeFile(join(archiveDirectory, config.name), `#!/bin/sh\nprintf '%s\\n' '${config.name} ${version.slice(1)}'\n`, { mode: 0o755 });
  await writeFile(join(archiveDirectory, "LICENSE"), "test license\n");
  await writeFile(join(archiveDirectory, "README.txt"), "test archive\n");
  const archivePath = join(fixtureDirectory, archive);
  const tarResult = spawnSync("tar", ["-czf", archivePath, "-C", archiveDirectory, config.name, "LICENSE", "README.txt"], { encoding: "utf8" });
  assert.equal(tarResult.status, 0, tarResult.stderr);

  const digest = createHash("sha256").update(await readFile(archivePath)).digest("hex");
  const expectedDigest = corruptChecksum ? "0".repeat(64) : digest;
  await writeFile(join(fixtureDirectory, "SHA256SUMS"), `${expectedDigest}  ${archive}\n`);

  const curlStub = `#!/bin/sh
output=
url=
write_out=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) output=$2; shift 2 ;;
    -w) write_out=$2; shift 2 ;;
    https://*) url=$1; shift ;;
    *) shift ;;
  esac
done
printf '%s\\n' "$url" >> "$LAZYAWS_TEST_CURL_LOG"
if [ "$url" = "$LAZYAWS_TEST_LATEST_URL" ]; then
  [ "$write_out" = '%{url_effective}' ] || exit 22
  printf '%s' "$LAZYAWS_TEST_LATEST_TAG_URL"
  exit 0
fi
case "$url" in
  "$LAZYAWS_TEST_DOWNLOAD_BASE/$LAZYAWS_TEST_VERSION/SHA256SUMS") source_file=$LAZYAWS_TEST_FIXTURE_DIR/SHA256SUMS ;;
  "$LAZYAWS_TEST_DOWNLOAD_BASE/$LAZYAWS_TEST_VERSION/$LAZYAWS_TEST_ARCHIVE") source_file=$LAZYAWS_TEST_FIXTURE_DIR/$LAZYAWS_TEST_ARCHIVE ;;
  *) exit 22 ;;
esac
[ -n "$output" ] && [ -f "$source_file" ] || exit 22
cp "$source_file" "$output"
`;
  const unameStub = `#!/bin/sh
case "$1" in
  -s) printf '%s\\n' "$LAZYAWS_TEST_UNAME_S" ;;
  -m) printf '%s\\n' "$LAZYAWS_TEST_UNAME_M" ;;
  *) exit 1 ;;
esac
`;
  const awkStub = `#!/bin/sh
if [ "$3" = /etc/os-release ]; then
  case "$2" in
    *VERSION_ID*) printf '%s\\n' "$LAZYAWS_TEST_OS_VERSION" ;;
    *\\\"ID\\\"*) printf '%s\\n' "$LAZYAWS_TEST_OS_ID" ;;
    *) exit 1 ;;
  esac
  exit 0
fi
exec "$LAZYAWS_TEST_REAL_AWK" "$@"
`;
  await Promise.all([
    writeFile(join(binDirectory, "curl"), curlStub, { mode: 0o755 }),
    writeFile(join(binDirectory, "uname"), unameStub, { mode: 0o755 }),
    writeFile(join(binDirectory, "awk"), awkStub, { mode: 0o755 }),
  ]);
  return {
    directory,
    homeDirectory,
    curlLog,
    env: {
      PATH: `${binDirectory}${delimiter}${process.env.PATH}`,
      HOME: homeDirectory,
      TMPDIR: directory,
      LAZYAWS_TEST_REAL_AWK: executableInPath("awk"),
      LAZYAWS_TEST_CURL_LOG: curlLog,
      LAZYAWS_TEST_FIXTURE_DIR: fixtureDirectory,
      LAZYAWS_TEST_LATEST_URL: config.latestUrl,
      LAZYAWS_TEST_LATEST_TAG_URL: latestTagUrl,
      LAZYAWS_TEST_DOWNLOAD_BASE: config.downloadBaseUrl,
      LAZYAWS_TEST_VERSION: version,
      LAZYAWS_TEST_ARCHIVE: archive,
      LAZYAWS_TEST_UNAME_S: "Linux",
      LAZYAWS_TEST_UNAME_M: "x86_64",
      LAZYAWS_TEST_OS_ID: "ubuntu",
      LAZYAWS_TEST_OS_VERSION: "24.04",
    },
  };
}

function runInstaller(testFixture, args = [], overrides = {}) {
  const result = spawnSync("/bin/sh", [installerPath, ...args], {
    cwd: root,
    env: { ...process.env, ...testFixture.env, ...overrides },
    encoding: "utf8",
  });
  if (result.error) throw result.error;
  return result;
}

test("generated installer matches config and has valid POSIX shell syntax", async () => {
  const generated = await renderInstaller();
  assert.equal(generated, await readFile(installerPath, "utf8"));
  assert.doesNotMatch(generated, /@@[^@]+@@/);
  const syntax = spawnSync("/bin/sh", ["-n", installerPath], { encoding: "utf8" });
  assert.equal(syntax.status, 0, syntax.stderr);
});

test("downloads the latest release and installs it to the default directory", async (t) => {
  const testFixture = await fixture();
  t.after(() => rm(testFixture.directory, { recursive: true, force: true }));

  const result = runInstaller(testFixture);
  const destination = join(testFixture.homeDirectory, ".local", "bin", config.name);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, new RegExp(`Installed ${config.name} ${version.slice(1)}`));
  assert.equal((await readFile(destination, "utf8")).includes(`${config.name} ${version.slice(1)}`), true);
  assert.deepEqual((await readFile(testFixture.curlLog, "utf8")).trim().split("\n"), [
    config.latestUrl,
    `${config.downloadBaseUrl}/${version}/SHA256SUMS`,
    `${config.downloadBaseUrl}/${version}/${archive}`,
  ]);

  await writeFile(testFixture.curlLog, "");
  const pinned = runInstaller(testFixture, [version]);
  assert.equal(pinned.status, 0, pinned.stderr);
  assert.deepEqual((await readFile(testFixture.curlLog, "utf8")).trim().split("\n"), [
    `${config.downloadBaseUrl}/${version}/SHA256SUMS`,
    `${config.downloadBaseUrl}/${version}/${archive}`,
  ]);
});

test("a latest release redirect outside this repository is refused", async (t) => {
  const testFixture = await fixture({ latestTagUrl: `https://github.com/someone/else/releases/tag/${version}` });
  t.after(() => rm(testFixture.directory, { recursive: true, force: true }));

  const result = runInstaller(testFixture);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /latest release response is invalid/);
  assert.deepEqual((await readFile(testFixture.curlLog, "utf8")).trim().split("\n"), [config.latestUrl]);
});

test("checksum failure preserves an installed binary", async (t) => {
  const testFixture = await fixture({ corruptChecksum: true });
  t.after(() => rm(testFixture.directory, { recursive: true, force: true }));
  const installDirectory = join(testFixture.directory, "existing install with spaces");
  const destination = join(installDirectory, config.name);
  const oldContents = "#!/bin/sh\nprintf 'old version\\n'\n";
  await mkdir(installDirectory);
  await writeFile(destination, oldContents, { mode: 0o755 });

  const result = runInstaller(testFixture, [], { LAZYAWS_INSTALL_DIR: installDirectory });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /SHA-256 checksum mismatch/);
  assert.equal(await readFile(destination, "utf8"), oldContents);
});

test("unsupported operating systems fail before network access", async (t) => {
  const testFixture = await fixture();
  t.after(() => rm(testFixture.directory, { recursive: true, force: true }));

  const result = runInstaller(testFixture, [], { LAZYAWS_TEST_UNAME_S: "FreeBSD" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /operating system is not supported: FreeBSD/);
  assert.equal(existsSync(testFixture.curlLog), false);
});
