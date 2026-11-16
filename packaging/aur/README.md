# Arch package: pryn-bin

The v0.1.1 release contains generated `pryn-bin.PKGBUILD` and `pryn-bin.SRCINFO` files. The recipe installs the prebuilt static-musl executable; it does not compile PRYN from Rust source and does not require Cargo.

**Status:** the recipe has not yet been submitted to the AUR. Until the AUR package page exists, `yay -S pryn-bin` is not a working installation method.

## Install the local package now

Install Arch's package-building tools:

```bash
sudo pacman -S --needed base-devel curl
```

Download the recipe from the published release:

```bash
mkdir -p ~/build/pryn-bin
cd ~/build/pryn-bin

curl -fL https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD -o PKGBUILD
```

Validate its source and build/install the package:

```bash
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
makepkg -si
```

The PKGBUILD contains a pinned SHA-256 for the matching GitHub release archive. `makepkg` verifies the downloaded archive before installing it.

## Submit to the AUR

After the GitHub release archive is public and local package validation passes:

1. Create the AUR package `pryn-bin` through your AUR account.
2. Clone the new AUR Git repository.
3. Copy the reviewed `PKGBUILD` and `.SRCINFO` into it.
4. Commit and push the package files.
5. Open the public package page and verify the package is searchable before changing the main README or site to advertise `yay -S pryn-bin`.

AUR packages are community-maintained, not official Arch repository packages. Keep the PKGBUILD source checksum synchronized with every release.
