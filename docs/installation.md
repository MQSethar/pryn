# Install PRYN

**Current release: v0.1.1 · Linux x86_64 · prebuilt packages need no Rust or Cargo.**

The easiest route is the [PRYN installation website](https://mqsethar.github.io/pryn/). Choose your Linux family, download the file, then copy the install command shown for it. The command installs the file you downloaded; it does not download the same file again.

All files are attached to the [v0.1.1 GitHub release](https://github.com/MQSethar/pryn/releases/tag/v0.1.1).

## Before you start

Check your machine architecture:

```bash
uname -m
```

It must print `x86_64`. ARM64/aarch64 is not available yet.

Download the package using your browser. Unless you changed your browser's download location, it will be in `~/Downloads`. The commands below assume that location.

## Debian, Ubuntu, Kali and compatible derivatives

Download `pryn_0.1.1_amd64.deb`, then install it:

```bash
cd ~/Downloads && sudo apt install ./pryn_0.1.1_amd64.deb
```

The `.deb` was installed and executed in Ubuntu 26.04, Debian 13 and Kali Rolling CI containers. Linux Mint, Pop!_OS and other Debian/Ubuntu derivatives are expected to be compatible, but have not all been individually tested.

This installs a local package. It does not add a PRYN APT repository, so `sudo apt install pryn` without the downloaded file is not available yet.

## Fedora and compatible RPM distributions

Download `pryn-0.1.1-1.x86_64.rpm`, then install it:

```bash
cd ~/Downloads && sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm
```

The RPM is unsigned. `--nogpgcheck` skips package-signature verification for this local installation. Install it only when you trust the release source and have checked the published checksum. Fedora passed the CI container smoke test. RHEL, Rocky Linux and AlmaLinux are intended targets, not individually tested systems yet.

### openSUSE

Download the same RPM and run:

```bash
cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./pryn-0.1.1-1.x86_64.rpm
```

The package is unsigned. This is the documented local-install path, but openSUSE is not currently in the automated smoke-test matrix.

## Arch Linux

The `pryn-bin` AUR package has **not** been published yet, so `yay -S pryn-bin` is not available.

You can install the prebuilt binary through the release PKGBUILD. Download `pryn-bin.PKGBUILD`, then run:

```bash
sudo pacman -S --needed base-devel
mkdir -p ~/build/pryn-bin
cp ~/Downloads/pryn-bin.PKGBUILD ~/build/pryn-bin/PKGBUILD
cd ~/build/pryn-bin
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
makepkg -si
```

This builds an Arch package around the precompiled PRYN binary. It does not compile PRYN from Rust source and does not need Cargo. `makepkg` checks the source archive against the checksum pinned in the PKGBUILD.

After the package is submitted and visible in the [AUR package index](https://aur.archlinux.org/packages/?K=pryn-bin), the normal AUR-helper installation will be:

```bash
yay -S pryn-bin
```

## Alpine and other compatible x86_64 Linux distributions

Download `pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz`, then extract and install it for your user:

```bash
cd ~/Downloads && tar -xzf pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz && install -Dm755 pryn-v0.1.1/pryn ~/.local/bin/pryn
```

Check it with:

```bash
~/.local/bin/pryn --version
```

If you want to run `pryn` without typing the full path, make sure `~/.local/bin` is in your shell's `PATH`. The archive passed the Alpine container startup test; Alpine hardware and service integration has not yet been validated on a physical Alpine machine.

## Verify a downloaded file (optional)

Download `SHA256SUMS` from the release and run:

```bash
cd ~/Downloads && curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/SHA256SUMS && sha256sum --ignore-missing --check SHA256SUMS
```

This checks any PRYN release files present in `~/Downloads`. Checksums detect accidental corruption. Because the checksum file and packages are hosted in the same GitHub release, this is not an independent publisher signature. The current release packages are unsigned.

## Check that PRYN works

After installation:

```bash
pryn --version
pryn check
```

To inspect one area:

```bash
pryn check wifi
pryn check bluetooth
pryn check audio
```

To print detailed evidence or JSON:

```bash
pryn check --verbose
pryn check --format json
```

PRYN is read-only and does not need `sudo` to run its supported checks. Exit code `1` means the report contains a warning, problem, unknown, partial or unimplemented result; it does not necessarily mean the program crashed.

## Updating and uninstalling

For a future release, download the new version's package and install it over the current version using the same command with the new filename. Direct-download packages do not update automatically. APT/RPM repositories are not provided yet.

Remove PRYN with the package manager used to install it:

```bash
sudo apt remove pryn       # Debian / Ubuntu / Kali
sudo dnf remove pryn       # Fedora
sudo zypper remove pryn     # openSUSE
sudo pacman -R pryn-bin     # local Arch package
```

For the generic archive:

```bash
rm -f ~/.local/bin/pryn
```

## What PRYN does not install

PRYN only inspects host services. These are optional and are not bundled with the executable:

| Host component | What PRYN can inspect |
|---|---|
| NetworkManager or standalone iwd | Wi-Fi manager state |
| BlueZ | Bluetooth adapter and device state |
| ALSA and a PulseAudio-compatible socket | Audio hardware and server details |
| `pw-dump` | Additional native PipeWire graph details |
| `ip` from iproute2 | IP addresses and routes |

A missing service does not prevent PRYN from installing or starting. Its related checks may be reported as unavailable or unknown. See the [support matrix](support-matrix.md) for the difference between container smoke tests and real-machine validation.
