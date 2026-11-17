# Arch User Repository: `pryn-bin`

## Package name decision

Keep the AUR package name **`pryn-bin`**. Arch's AUR submission guidance says packages using prebuilt deliverables must use the `-bin` suffix when source is available. The suffix describes how the package is delivered; the installed executable and user command remain **`pryn`**.

The v0.1.1 release contains:

- [`pryn-bin.PKGBUILD`](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD)
- [`pryn-bin.SRCINFO`](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.SRCINFO)
- [Static x86_64 archive](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz)

**Current status:** not yet submitted. Do not advertise `yay -S pryn-bin` as available until the AUR package page exists.

## 1. Prepare an AUR account

1. Register at <https://aur.archlinux.org/register>.
2. Verify your account email.
3. Open your AUR account settings and add an SSH public key. Do not upload or share the private key.

Use a dedicated key for the AUR so it can be revoked independently from GitHub. On your Arch machine:

```bash
ssh-keygen -t ed25519 -C "pryn AUR upload" -f ~/.ssh/aur_pryn
```

Choose a passphrase when prompted. Add the contents of `~/.ssh/aur_pryn.pub` to your AUR account's SSH Public Key field:

```bash
cat ~/.ssh/aur_pryn.pub
```

Add this host entry to `~/.ssh/config` without deleting your existing GitHub entries:

```sshconfig
Host aur.archlinux.org
    IdentityFile ~/.ssh/aur_pryn
    User aur
    IdentitiesOnly yes
```

Set safe permissions and test authentication:

```bash
chmod 700 ~/.ssh
chmod 600 ~/.ssh/config ~/.ssh/aur_pryn
chmod 644 ~/.ssh/aur_pryn.pub
ssh -T aur@aur.archlinux.org
```

The AUR server does not provide an interactive shell. A successful authentication message is what matters.

## 2. Test the exact release PKGBUILD before submission

Install the standard build tools if they are missing:

```bash
sudo pacman -S --needed base-devel curl git
```

Create a clean test directory and download the published release recipe:

```bash
mkdir -p ~/Projects/pryn-aur-test
cd ~/Projects/pryn-aur-test
curl -fL https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD -o PKGBUILD
```

Generate current AUR metadata, verify the source checksum, then build and install:

```bash
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
makepkg --cleanbuild --syncdeps --install
```

Verify that the installed command exists and reports the expected version:

```bash
pryn --version
command -v pryn
pacman -Q pryn-bin
```

Expected: version `0.1.1`, executable path `/usr/bin/pryn`, and installed package name `pryn-bin`.

Do not submit the package until this clean build/install succeeds.

## 3. Create the AUR package repository

The package name is not currently listed in the AUR. Clone the new package repository using the AUR SSH endpoint:

```bash
mkdir -p ~/Projects/aur
cd ~/Projects/aur
git clone ssh://aur@aur.archlinux.org/pryn-bin.git
cd pryn-bin
```

For a new package, an empty-repository warning during clone is expected. If SSH authentication fails, fix the AUR SSH key setup first; do not switch to a password or put a private key in the repository.

## 4. Add only the package files

Download the exact release PKGBUILD and regenerate `.SRCINFO` from it:

```bash
curl -fL https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD -o PKGBUILD
makepkg --printsrcinfo > .SRCINFO
```

Review both files:

```bash
less PKGBUILD
less .SRCINFO
```

The source URL must point to the v0.1.1 release archive and the SHA-256 must match that archive. Do not upload the binary tarball or a package archive to the AUR. The AUR stores the PKGBUILD and metadata; users build the package locally with makepkg.

## 5. Commit and push to AUR

The AUR expects the branch named `master`:

```bash
git add PKGBUILD .SRCINFO
git diff --cached --check
git diff --cached --stat
git commit -m "Initial import: pryn-bin 0.1.1"
git push origin HEAD:master
```

Then open <https://aur.archlinux.org/packages/pryn-bin> and confirm the package page exists and the displayed package metadata says `0.1.1`.

## 6. Verify the public install route

After the AUR page exists, test from a clean Arch environment or ask another Arch user to run:

```bash
yay -S pryn-bin
pryn --version
```

Only after this test succeeds should `aurPublished` in `site/release.json` be changed from `false` to `true`. The website and README should then advertise `yay -S pryn-bin` as the standard Arch route.

## 7. Future releases

For each PRYN release:

1. Generate the new `pryn-bin.PKGBUILD` from the distribution workflow.
2. Verify that its release URL and SHA-256 match the new static archive.
3. Update `PKGBUILD` in the AUR repository and regenerate `.SRCINFO` using `makepkg --printsrcinfo > .SRCINFO`.
4. Commit and push both files to `master`.
5. Test the upgrade with `yay -Syu` or `yay -S pryn-bin`.

Do not change `pkgver` or `pkgrel` for a README typo or other metadata-only edit that does not alter the packaged result. Keep the package version synchronized with the actual upstream asset.

## References

- <https://wiki.archlinux.org/title/AUR_submission_guidelines>
- <https://wiki.archlinux.org/title/PKGBUILD>
- <https://wiki.archlinux.org/title/.SRCINFO>
