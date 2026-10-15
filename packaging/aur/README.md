# AUR package

`lazyaws-bin` packages the x86_64 Linux release archive for Arch Linux and
Omarchy. The recipe installs the binary, license, and release README. It does
not run the curl installer. It is prepared locally and is not published to AUR.

After the release archive and its `SHA256SUMS` are public, run this from the
repository root:

```sh
node scripts/generate-packages.mjs /path/to/SHA256SUMS
makepkg --printsrcinfo --dir packaging/aur/lazyaws-bin > packaging/aur/lazyaws-bin/.SRCINFO
```

Build and review the package as a normal user in a clean Arch environment.
Then clone the AUR repository, copy `PKGBUILD` and `.SRCINFO` into it, and
publish the update:

```sh
git -c init.defaultBranch=master clone ssh://aur@aur.archlinux.org/lazyaws-bin.git
cp packaging/aur/lazyaws-bin/PKGBUILD packaging/aur/lazyaws-bin/.SRCINFO lazyaws-bin/
cd lazyaws-bin
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Update lazyaws-bin to <version>"
git push
```

Do not run these publication commands until the release archive is available
and the AUR maintainer SSH key is configured.
