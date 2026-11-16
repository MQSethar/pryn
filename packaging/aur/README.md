# AUR package: pryn-bin

The release workflow generates `dist/pryn-bin.PKGBUILD` and
`dist/pryn-bin.SRCINFO` for the exact release archive and checksum. Do not
submit the template by itself: use the generated files from the release you
are publishing.

Install the standard Arch package build tools once if they are missing:

```bash
sudo pacman -S --needed base-devel git
```

On Arch Linux, test the generated package before submitting it:

```bash
mkdir -p ~/Projects/pryn-aur-test
cp dist/pryn-bin.PKGBUILD ~/Projects/pryn-aur-test/PKGBUILD
cd ~/Projects/pryn-aur-test
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
makepkg --cleanbuild --syncdeps --install
```

After the local build and install test pass, create the AUR package repository
through your AUR account, clone it, copy the tested `PKGBUILD` and `.SRCINFO`,
then commit and push. AUR is community-maintained; it is not an official Arch
repository.
