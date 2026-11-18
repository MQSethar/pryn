# Install, update and remove PRYN

**Current release: v0.1.1 · Linux x86_64 / amd64**

The easiest way to install PRYN is through the [PRYN installation website](https://pryn.qasimio.me/). Choose your Linux family, download the package, then copy the command shown for that package.

Prebuilt packages do not require Rust or Cargo. ARM64/aarch64 is not supported yet.

All prebuilt files are attached to the [v0.1.1 GitHub release](https://github.com/MQSethar/pryn/releases/tag/v0.1.1).

## Before you start

Check your architecture:

```bash
uname -m
```

It must print `x86_64`.

Unless you choose another download location, save release files in `~/Downloads`. Commands below assume that location.

## Debian, Ubuntu, Kali and compatible derivatives

### Install

Download `pryn_0.1.1_amd64.deb`, then run:

```bash
cd ~/Downloads && sudo apt install ./pryn_0.1.1_amd64.deb
```

The `.deb` passed container installation tests on Ubuntu 26.04, Debian 13 and Kali Rolling. Linux Mint and Pop!_OS are expected to be compatible but have not been individually tested.

### Update

Download the newer `.deb` from the latest release, then run the same command with its new filename:

```bash
cd ~/Downloads && sudo apt install ./pryn_0.1.1_amd64.deb
```

APT upgrades the installed package when the downloaded package has a newer version. Direct downloads do not update automatically.

### Remove

```bash
sudo apt remove pryn
```

This removes the package but keeps system configuration files, if any.

## Fedora and compatible RPM distributions

### Install

Download `pryn-0.1.1-1.x86_64.rpm`, then run:

```bash
cd ~/Downloads && sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm
```

The current RPM is unsigned. `--nogpgcheck` disables signature verification for this local installation command. Only install it if you trust the GitHub release and have checked the published SHA-256 checksum.

Fedora passed the CI container smoke test. RHEL, Rocky Linux and AlmaLinux are intended targets but have not been individually tested.

### Update

Download the newer RPM, then run the same command with its new filename:

```bash
cd ~/Downloads && sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm
```

DNF installs the local package and upgrades the installed PRYN package when the downloaded version is newer.

### Remove

```bash
sudo dnf remove pryn
```

## openSUSE

openSUSE uses the same x86_64 RPM. It is an intended target, but is not currently in the automated smoke-test matrix.

### Install

Download `pryn-0.1.1-1.x86_64.rpm`, then run:

```bash
cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./pryn-0.1.1-1.x86_64.rpm
```

The RPM is unsigned. `--allow-unsigned-rpm` allows this local package to be installed.

### Update

Download the newer RPM and run the same local install command with its new filename:

```bash
cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./pryn-0.1.1-1.x86_64.rpm
```

### Remove

```bash
sudo zypper remove pryn
```

## Arch Linux

The `pryn-bin` AUR package has not been published yet. Until it is available in the AUR, install the prebuilt binary using the maintained [PRYN PKGBUILD](https://pryn.qasimio.me/assets/pryn-bin.PKGBUILD).

### Install

Download `pryn-bin.PKGBUILD` from the [PRYN website](https://pryn.qasimio.me/assets/pryn-bin.PKGBUILD), then run:

```bash
sudo pacman -S --needed base-devel
mkdir -p ~/build/pryn-bin
cp ~/Downloads/pryn-bin.PKGBUILD ~/build/pryn-bin/PKGBUILD
cd ~/build/pryn-bin
makepkg -si
```

`makepkg` verifies the source archive against the checksum pinned in the PKGBUILD. It packages the already-built PRYN executable; it does not compile PRYN from Rust source.

### Update

Download the latest `pryn-bin.PKGBUILD` from the website, replace the old recipe, then run:

```bash
cp -f ~/Downloads/pryn-bin.PKGBUILD ~/build/pryn-bin/PKGBUILD
cd ~/build/pryn-bin
makepkg -si
```

Use the same directory created during installation. If your browser renames a repeated download (for example, `pryn-bin (1).PKGBUILD`), rename it to `pryn-bin.PKGBUILD` before running the command.

### Remove

```bash
sudo pacman -Rns pryn-bin
```

Once `pryn-bin` is actually published in the AUR, the shorter commands will be:

```bash
yay -S pryn-bin       # install
yay -Syu pryn-bin     # update
yay -Rns pryn-bin     # remove
```

Do not use these AUR commands until the package page exists: https://aur.archlinux.org/packages/pryn-bin

## Alpine and other compatible x86_64 Linux

Use the static-musl archive when a native package is not available.

### Install

Download `pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz`, then run:

```bash
mkdir -p ~/.local/bin
tar -xzf ~/Downloads/pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz -C /tmp
install -Dm755 /tmp/pryn-v0.1.1/pryn ~/.local/bin/pryn
```

### Update

Download the newer archive, then run the same extraction and install steps using its versioned directory and filename. The final `install` command replaces the executable in `~/.local/bin`.

### Remove

```bash
rm -f ~/.local/bin/pryn
```

If `pryn` is not found after installation, run `~/.local/bin/pryn --version` or ensure `~/.local/bin` is on your shell's `PATH`.

The archive passed an Alpine container startup test. Physical Alpine hardware and service integration have not been validated.

## Install, update or remove with Cargo

Use Cargo if you already have Rust installed or specifically want Cargo to manage PRYN. Cargo compiles the published crate on your machine. It requires Rust and Cargo; this is not the prebuilt-package route.

Run these commands as your normal user. Do not use `sudo`.

### Install

```bash
cargo install pryn --locked
```

### Update

```bash
cargo install pryn --locked --force
```

`--force` replaces the installed Cargo-managed binary with the latest published crate version.

### Remove

```bash
cargo uninstall pryn
```

Cargo normally places the executable in `~/.cargo/bin/pryn`. Make sure that directory is on your `PATH`.

## Check that PRYN works

After installation:

```bash
pryn --version
pryn check
```

PRYN does not need `sudo` for its supported checks.

Exit code `1` means the report was completed but contains a warning, problem, unknown, partial or unimplemented result. It does not necessarily mean the program crashed.

## Important: remove PRYN using the same method you installed it with

| Installed with | Update with | Remove with |
|---|---|---|
| APT `.deb` | `sudo apt install ./new-file.deb` | `sudo apt remove pryn` |
| DNF `.rpm` | `sudo dnf install --nogpgcheck ./new-file.rpm` | `sudo dnf remove pryn` |
| Zypper `.rpm` | `sudo zypper install --allow-unsigned-rpm ./new-file.rpm` | `sudo zypper remove pryn` |
| Local Arch PKGBUILD | Download new PKGBUILD, then `makepkg -si` | `sudo pacman -Rns pryn-bin` |
| AUR, when published | `yay -Syu pryn-bin` | `yay -Rns pryn-bin` |
| Static archive | Replace `~/.local/bin/pryn` | `rm -f ~/.local/bin/pryn` |
| Cargo | `cargo install pryn --locked --force` | `cargo uninstall pryn` |

Do not mix installation methods. For example, `cargo uninstall pryn` does not remove an APT-managed or pacman-managed package.

## Optional: verify the downloaded file

The release includes `SHA256SUMS`. Download the checksum file and verify the assets you downloaded:

```bash
cd ~/Downloads
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/SHA256SUMS
sha256sum --ignore-missing --check SHA256SUMS
```

Checksums detect accidental corruption. Because the checksum and packages are hosted in the same release, this is not an independent publisher signature. The current release packages are unsigned.

## What PRYN does not install

PRYN observes host services; it does not install or manage them.

| Optional host component | What PRYN can inspect |
|---|---|
| NetworkManager or standalone iwd | Wi-Fi manager state |
| BlueZ | Bluetooth adapter and device state |
| ALSA and a PulseAudio-compatible socket | Audio hardware and server details |
| `pw-dump` | Additional native PipeWire graph details |
| `ip` from iproute2 | IP addresses and routes |
| `/etc/resolv.conf` | Configured DNS nameservers |

A missing service does not prevent PRYN from installing or starting. Related checks may be unavailable or unknown. See the [support matrix](support-matrix.md) for the difference between container smoke tests and real-machine validation.
