# Install PRYN on Linux

**Current release: v0.1.1 · x86_64 only**

This guide is for people who want to install and run PRYN without installing Rust, Cargo, or a compiler. Use a prebuilt release asset, not `cargo install`.

All release files are available on the [PRYN v0.1.1 GitHub release](https://github.com/MQSethar/pryn/releases/tag/v0.1.1).

## 1. Before you download

- **Architecture:** your system must be x86_64/amd64. ARM64/aarch64 is not provided yet.
- **Internet:** you need access to GitHub Releases to download the files.
- **Privileges:** package installation needs administrator privileges; running `pryn check` does not.
- **Host services:** PRYN does not install NetworkManager, iwd, BlueZ, PipeWire, PulseAudio, ALSA utilities, or iproute2. These are inspected as optional host capabilities. Missing services result in unavailable/unknown checks, not an installation failure.
- **Package trust:** the release publishes SHA-256 checksums, but release packages are not cryptographically signed. The RPM is unsigned, so local RPM installation requires bypassing package-signature checks. Read the security note below.

Check architecture:

```bash
uname -m
```

Expected output:

```text
x86_64
```

## 2. Choose your distribution

### Debian, Ubuntu, Kali, Linux Mint, Pop!_OS and compatible derivatives

Use the `.deb` package. The current v0.1.1 asset is:

[`pryn_0.1.1_amd64.deb`](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn_0.1.1_amd64.deb)

Download and install:

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn_0.1.1_amd64.deb
sudo apt install ./pryn_0.1.1_amd64.deb
```

APT installs the local package file. This is not the same as adding a PRYN APT repository: `sudo apt install pryn` by itself will not work from the standard Ubuntu/Debian repositories.

The package was installed and executed in Ubuntu 26.04, Debian 13, and Kali Rolling CI containers. Linux Mint and Pop!_OS are intended compatible Debian/Ubuntu derivatives, but have not been individually smoke-tested.

### Fedora

Use the `.rpm` package:

[`pryn-0.1.1-1.x86_64.rpm`](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-0.1.1-1.x86_64.rpm)

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-0.1.1-1.x86_64.rpm
sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm
```

The RPM is unsigned. `--nogpgcheck` tells DNF not to require a package signature for this local installation. Only install it if you trust the release source and have checked the published checksum.

The package was installed and executed in a Fedora container. Rocky Linux, AlmaLinux, and RHEL are intended RPM-family targets, not individually validated systems yet.

### openSUSE

Use the same x86_64 `.rpm` file:

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-0.1.1-1.x86_64.rpm
sudo zypper install --allow-unsigned-rpm ./pryn-0.1.1-1.x86_64.rpm
```

The release RPM is unsigned. The openSUSE command is documented as the intended local-install path, but openSUSE is not currently in the automated smoke-test matrix. Review the package signature warning before proceeding.

### Arch Linux

The release contains a generated AUR binary recipe, but **`pryn-bin` has not been submitted to the AUR yet**. Therefore this will not work today:

```bash
yay -S pryn-bin
```

You can still build a local Arch package from the precompiled static binary. This uses `makepkg`, not Cargo, and does not compile PRYN from Rust source.

Install the standard package-building tools:

```bash
sudo pacman -S --needed base-devel curl
```

Download the release recipe and build it:

```bash
mkdir -p ~/build/pryn-bin
cd ~/build/pryn-bin

curl -fL https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD -o PKGBUILD

makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
makepkg -si
```

`makepkg` downloads the v0.1.1 static archive and verifies it against the SHA-256 pinned in the PKGBUILD. The package will then be installed and managed by pacman.

Once the package has actually been submitted and is visible in the [AUR package index](https://aur.archlinux.org/packages/?K=pryn-bin), the short installation command will be:

```bash
yay -S pryn-bin
```

Do not advertise that command as available before the AUR page exists.

### Alpine Linux and other compatible x86_64 distributions

Use the universal static-musl archive:

[`pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz`](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz)

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz

tar -xzf pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz

mkdir -p "$HOME/.local/bin"
install -m 755 pryn-v0.1.1/pryn "$HOME/.local/bin/pryn"
```

Check it:

```bash
"$HOME/.local/bin/pryn" --version
```

Make sure `$HOME/.local/bin` is on your `PATH`. If it is not, add this line to the startup file for your shell:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

The static archive passed the Alpine container startup smoke test. Alpine's system services and hardware integrations have not been validated on a physical Alpine machine.

## 3. Verify the download checksum

The release includes a `SHA256SUMS` file. Download it beside the asset:

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/SHA256SUMS
sha256sum --ignore-missing --check SHA256SUMS
```

This checks whichever release assets are present in the current directory and ignores entries for files you did not download.

A checksum protects against accidental corruption. Because the checksum file and packages are hosted in the same GitHub release, it does not independently prove who published them. Cryptographic release signing and hosted signed package repositories are deferred.

## 4. Confirm that PRYN works

After installation:

```bash
pryn --version
pryn --help
pryn check
```

To inspect a specific subsystem:

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

PRYN is read-only. It does not need `sudo` for these checks. Exit code `1` means the report contains a warning, problem, unknown, partial, or unimplemented result; it does not necessarily mean the program crashed.

## 5. Update and uninstall

### Updating

For v0.1.1, download the new release package and install it over the existing package using the same package-manager command. For the generic archive, replace the old `pryn` executable with the new one.

For future versions, use the matching versioned asset and checksum from that release. Direct-download packages do not automatically update themselves. An APT/RPM repository is not available yet.

### Uninstalling

Debian/Ubuntu/Kali:

```bash
sudo apt remove pryn
```

Fedora:

```bash
sudo dnf remove pryn
```

openSUSE:

```bash
sudo zypper remove pryn
```

Arch local package:

```bash
sudo pacman -R pryn-bin
```

Generic archive installed under `~/.local/bin`:

```bash
rm -f "$HOME/.local/bin/pryn"
```

## 6. What is and is not installed

The PRYN package contains the PRYN executable and documentation. It does not install or enable host services.

| Capability PRYN inspects | Host component | Required to install PRYN? |
|---|---|---|
| Wi-Fi manager state | NetworkManager or standalone iwd over system D-Bus | No |
| Bluetooth state | BlueZ over system D-Bus | No |
| Audio server details | PulseAudio-compatible user socket | No |
| Native PipeWire graph details | `pw-dump` | No; optional enhancement |
| IP addresses and routes | `ip` from iproute2 | No; those observations may be unknown without it |
| DNS configuration | `/etc/resolv.conf` | No |

A successful package installation does not guarantee every diagnostic category will be available on that machine.

## Support notes

Automated container smoke tests are recorded in [the support matrix](support-matrix.md). Container tests validate package installation and process startup; they do not certify physical Wi-Fi, Bluetooth, audio, or internet behavior on every distribution.
