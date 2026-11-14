# Releasing PRYN

This document describes the first public release. The first release is intentionally narrow: one Linux x86_64 GNU binary, one crates.io package, and one GitHub Release. Do not expand supported architectures or add new product features as part of the v0.1.0 release.

## Release gates

Do not tag or publish until all of these are true:

1. `./scripts/verify.sh` passes on the pinned toolchain. It runs formatting, strict Clippy, all tests, a release build, and `cargo package --locked`.
2. `cargo package --list` contains the expected source, README, changelog, and license files, with no secrets, local reports, or development-only files.
3. `cargo publish --dry-run --locked` succeeds.
4. GitHub Actions is green on the exact commit that will be tagged.
5. Wi-Fi and Bluetooth are validated on the maintainer's machine. Audio playback and microphone capture are manually tested. The standalone iwd path is separately tested before it is advertised as runtime-validated.
6. The repository is clean, the release notes are accurate, and all source contributions can be distributed under MIT OR Apache-2.0.

The latest supplied runtime log shows Wi-Fi and Bluetooth passing. Audio's physical port checks and active-profile availability conflict remain `unknown`. Do not suppress these states just to make the overall report green; document this limitation in the v0.1.0 notes.

## Check the crates.io name

Crate names are first-come, first-served. Before preparing the final tag, check whether `pryn` is available:

```bash
cargo search pryn --limit 10
```

Also inspect `https://crates.io/crates/pryn`. If another owner has already claimed the name, stop. Do not publish under a misleading or unrelated name; decide the package name deliberately and update the install instructions before release. The binary executable can still be named `pryn` even if the package name must differ, but that requires an explicit Cargo `[[bin]]` declaration.

## Prepare the crate

From the clean, verified release commit:

```bash
cargo package --list
cargo package --locked
cargo publish --dry-run --locked
```

Inspect the generated package under `target/package/`. Check that both `LICENSE-MIT` and `LICENSE-APACHE` are included and that the package contains only intended project files.

## Create the source tag and draft binary release

Merge the release commit into `main`, pull the merged `main`, and run the verification script once more. Then create and push the annotated tag:

```bash
git switch main
git pull --ff-only origin main
./scripts/verify.sh
git tag -a v0.1.0 -m "PRYN v0.1.0"
git push origin v0.1.0
```

The `PRYN Release` workflow runs from the tagged source. It verifies the tag, builds `x86_64-unknown-linux-gnu`, creates a tarball containing the binary, README, changelog, and both licenses, calculates `SHA256SUMS`, and opens a **draft** GitHub Release. It does not publish the draft automatically.

Review the draft release and download the archive. Verify its checksum, inspect its contents, then test the binary on a clean x86_64 GNU/Linux environment:

```bash
sha256sum --check SHA256SUMS
tar -tzf pryn-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

After downloading and extracting the archive, run `./pryn --help` and each of the documented `pryn check` commands. Confirm the executable reports version `0.1.0`.

## Publish to crates.io

First create an account at crates.io, verify the account email, and create a narrowly scoped API token. Do not commit the token or put it in a command that will be saved in shell history.

For a one-time local publish, read the token into an environment variable without echoing it:

```bash
read -rsp "crates.io API token: " CARGO_REGISTRIES_CRATES_IO_TOKEN
echo
export CARGO_REGISTRIES_CRATES_IO_TOKEN
cargo publish --locked
unset CARGO_REGISTRIES_CRATES_IO_TOKEN
```

Publishing a version to crates.io is effectively permanent: an uploaded version cannot be overwritten or deleted. A version can be yanked, but that is not a substitute for validating it before publication. Do not run `cargo publish` until the dry run, source review, release commit, and package-name check are complete.

## Publish the GitHub Release

Once the crates.io package is visible and the draft GitHub Release assets/checksums have been tested, publish the existing draft through the GitHub Releases page. Include the final changelog summary and note the initial target limitation: x86_64 Linux with GNU libc.

## Verify the public installation path

On a clean machine after crates.io publication:

```bash
cargo install pryn --locked
pryn --version
pryn --help
pryn check
```

For the binary archive, verify `SHA256SUMS` before extracting, then install the `pryn` executable into a directory on `PATH`, for example `~/.local/bin`.

## After release

Freeze feature work while collecting real usage reports. Prioritize only release-blocking bugs, compatibility defects, misleading diagnostics, and installation failures. Do not add a TUI, daemon, automatic repair, Bluetooth automation, or audio playback/recording to the v0.1 maintenance cycle.
