import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configPath = path.join(repositoryRoot, "distribution/config.json");
const cargoPath = path.join(repositoryRoot, "Cargo.toml");

function packageMetadata(cargoToml) {
  const packageSection = cargoToml.split(/^\[/m).find((section) => section.startsWith("package]"));
  const version = packageSection?.match(/^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$/m)?.[1];
  const description = packageSection?.match(/^description\s*=\s*"([^"\r\n]+)"\s*$/m)?.[1];
  const license = packageSection?.match(/^license\s*=\s*"([A-Za-z0-9.+-]+)"\s*$/m)?.[1];

  if (!version || !description || !license) {
    throw new Error("Cargo.toml must define a stable version, description, and SPDX license in [package]");
  }

  return { version, description, license };
}

function validateConfig(config) {
  for (const key of ["name", "repository", "homepage", "downloadBaseUrl", "tap", "aurPackage"]) {
    if (typeof config[key] !== "string" || config[key].length === 0) {
      throw new Error(`distribution/config.json must define ${key}`);
    }
  }

  if (!/^[A-Za-z0-9._-]+$/.test(config.name) || !/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(config.tap)) {
    throw new Error("distribution/config.json has an invalid package name or Homebrew tap");
  }

  if (!/^[A-Za-z0-9._-]+$/.test(config.aurPackage) || !/^[a-z_]+$/.test(config.minimumMacosRelease)) {
    throw new Error("distribution/config.json has an invalid AUR package name or Homebrew macOS release");
  }

  for (const key of ["homepage", "downloadBaseUrl"]) {
    const url = new URL(config[key]);
    if (url.protocol !== "https:" || !url.hostname) {
      throw new Error(`distribution/config.json ${key} must use HTTPS`);
    }
  }

  if (!/^\d+\.\d+(?:\.\d+)?$/.test(config.minimumUbuntuVersion) || !/^\d+\.\d+(?:\.\d+)?$/.test(config.minimumMacosVersion)) {
    throw new Error("distribution/config.json must define numeric minimum OS versions");
  }

  if (!Array.isArray(config.targets) || config.targets.length !== 4) {
    throw new Error("distribution/config.json must define exactly four release targets");
  }

  for (const target of config.targets) {
    if (!/^[A-Za-z0-9_-]+$/.test(target.triple) || !["darwin", "linux"].includes(target.os) || !["arm64", "aarch64", "x86_64"].includes(target.arch)) {
      throw new Error("distribution/config.json has an invalid release target");
    }
  }

  const targetPairs = new Set(config.targets.map((target) => `${target.os}:${target.arch}`));
  const requiredTargetPairs = ["darwin:arm64", "darwin:x86_64", "linux:aarch64", "linux:x86_64"];
  if (targetPairs.size !== requiredTargetPairs.length || requiredTargetPairs.some((pair) => !targetPairs.has(pair))) {
    throw new Error("distribution/config.json must define one archive for each supported OS and CPU pair");
  }
}

function parseChecksums(contents, archiveNames) {
  const expected = new Set(archiveNames);
  const checksums = new Map();

  for (const line of contents.split(/\r?\n/).filter((item) => item.length > 0)) {
    const match = line.match(/^([a-fA-F0-9]{64})  ([^/\\]+)$/);
    if (!match || !expected.has(match[2]) || checksums.has(match[2])) {
      throw new Error(`Invalid SHA256SUMS entry: ${line}`);
    }

    checksums.set(match[2], match[1].toLowerCase());
  }

  if (checksums.size !== expected.size || [...expected].some((name) => !checksums.has(name))) {
    throw new Error("SHA256SUMS must contain exactly one two-space SHA-256 entry for each release archive");
  }

  return checksums;
}

function archiveName(name, version, target) {
  return `${name}-v${version}-${target.triple}.tar.gz`;
}

function archiveUrl(config, version, name) {
  return `${config.downloadBaseUrl.replace(/\/+$/, "")}/v${version}/${name}`;
}

function renderFormula(config, metadata, targets, checksums) {
  const targetBlock = (cpu, target) => {
    const name = archiveName(config.name, metadata.version, target);
    return `    ${cpu} do\n      url ${JSON.stringify(archiveUrl(config, metadata.version, name))}\n      sha256 "${checksums.get(name)}"\n    end`;
  };
  const darwinArm = targets.find((target) => target.os === "darwin" && target.arch === "arm64");
  const darwinIntel = targets.find((target) => target.os === "darwin" && target.arch === "x86_64");
  const linuxArm = targets.find((target) => target.os === "linux" && target.arch === "aarch64");
  const linuxIntel = targets.find((target) => target.os === "linux" && target.arch === "x86_64");
  const platformBlocks = `  on_macos do\n    depends_on macos: :${config.minimumMacosRelease}\n${targetBlock("on_arm", darwinArm)}\n${targetBlock("on_intel", darwinIntel)}\n  end\n\n  on_linux do\n${targetBlock("on_arm", linuxArm)}\n${targetBlock("on_intel", linuxIntel)}\n  end`;

  return `class Lazyaws < Formula
  desc ${JSON.stringify(metadata.description)}
  homepage ${JSON.stringify(config.homepage)}
  version "${metadata.version}"
  license "${metadata.license}"

${platformBlocks}

  def install
    bin.install "lazyaws"
  end

  def caveats
    <<~EOS
      Normal AWS service use requires AWS CLI v2.
      Demo mode works without AWS CLI or credentials.

      Linux binaries target Ubuntu ${config.minimumUbuntuVersion} and systems with a compatible libc.
    EOS
  end

  test do
    assert_match "lazyaws #{version}", shell_output("#{bin}/lazyaws --version")
  end
end
`;
}

function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function renderAur(config, metadata, linuxTarget, checksum) {
  const name = archiveName(config.name, metadata.version, linuxTarget);
  const url = archiveUrl(config, metadata.version, name);
  const pkgdesc = shellQuote(metadata.description);
  const pkgurl = shellQuote(config.homepage);
  const pkglicense = shellQuote(metadata.license);

  const pkgbuild = `pkgname=${config.aurPackage}
pkgver=${metadata.version}
pkgrel=1
pkgdesc=${pkgdesc}
arch=('x86_64')
url=${pkgurl}
license=(${pkglicense})
depends=('glibc' 'gcc-libs')
provides=("${config.name}=\$pkgver")
conflicts=('${config.name}')
source=(${shellQuote(url)})
sha256sums=('${checksum}')

package() {
  install -Dm755 "$srcdir/lazyaws" "$pkgdir/usr/bin/lazyaws"
  install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 "$srcdir/README.txt" "$pkgdir/usr/share/doc/$pkgname/README.txt"
}
`;

  const srcinfo = `pkgbase = ${config.aurPackage}
\tpkgdesc = ${metadata.description}
\tpkgver = ${metadata.version}
\tpkgrel = 1
\turl = ${config.homepage}
\tarch = x86_64
\tlicense = ${metadata.license}
\tdepends = glibc
\tdepends = gcc-libs
\tprovides = ${config.name}=${metadata.version}
\tconflicts = ${config.name}
\tsource = ${url}
\tsha256sums = ${checksum}

pkgname = ${config.aurPackage}
`;

  return { pkgbuild, srcinfo };
}

export async function generatePackages(checksumsPath, outputDirectory = path.join(repositoryRoot, "packaging")) {
  const [configText, cargoText, checksumText] = await Promise.all([
    readFile(configPath, "utf8"),
    readFile(cargoPath, "utf8"),
    readFile(checksumsPath, "utf8"),
  ]);
  const config = JSON.parse(configText);
  validateConfig(config);
  const metadata = packageMetadata(cargoText);
  const checksums = parseChecksums(checksumText, config.targets.map((target) => archiveName(config.name, metadata.version, target)));
  const linuxX64 = config.targets.find((target) => target.os === "linux" && target.arch === "x86_64");
  const { pkgbuild, srcinfo } = renderAur(
    config,
    metadata,
    linuxX64,
    checksums.get(archiveName(config.name, metadata.version, linuxX64)),
  );
  const homebrewPath = path.join(outputDirectory, "homebrew/Formula/lazyaws.rb");
  const aurDirectory = path.join(outputDirectory, "aur", config.aurPackage);

  await Promise.all([mkdir(path.dirname(homebrewPath), { recursive: true }), mkdir(aurDirectory, { recursive: true })]);
  await Promise.all([
    writeFile(homebrewPath, renderFormula(config, metadata, config.targets, checksums)),
    writeFile(path.join(aurDirectory, "PKGBUILD"), pkgbuild),
    writeFile(path.join(aurDirectory, ".SRCINFO"), srcinfo),
  ]);

  return [homebrewPath, path.join(aurDirectory, "PKGBUILD"), path.join(aurDirectory, ".SRCINFO")];
}

async function main() {
  const [checksumsPath, outputDirectory, ...extraArgs] = process.argv.slice(2);
  if (!checksumsPath || extraArgs.length > 0) {
    throw new Error("Usage: node scripts/generate-packages.mjs SHA256SUMS [output-directory]");
  }

  const generated = await generatePackages(checksumsPath, outputDirectory ? path.resolve(outputDirectory) : undefined);
  process.stdout.write(`Generated ${generated.join(" and ")}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
