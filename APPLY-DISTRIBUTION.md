# PRYN portable distribution implementation

This patch adds the x86_64 static-musl distribution pipeline and package
wrappers. It does not change diagnostic logic. It does not publish anything,
create a Git tag, or sign packages.

## Important release decision

PRYN v0.1.0 is already published and must remain immutable. These distribution
changes should ship in **v0.1.1** after the new pipeline passes CI and the
maintainer manually tests the static binary on Arch, Ubuntu, Kali, and Alpine.
Do not retag or overwrite v0.1.0.

## 1. Start from main and create a branch

```bash
cd ~/Projects/pryn
git switch main
git pull --ff-only origin main
git status --short
git switch -c distribution/portable-x86_64
```

The working tree must be clean before creating the branch.

## 2. Apply this ZIP from the repository root

Download `pryn-distribution-kit.zip` into `~/Downloads`, then:

```bash
cd ~/Projects/pryn
unzip -t ~/Downloads/pryn-distribution-kit.zip
unzip -o ~/Downloads/pryn-distribution-kit.zip -d .
```

The ZIP contains new distribution scripts, packaging metadata, CI/release
workflows, and updated README/support documentation. It replaces
`.github/workflows/release.yml` with a workflow that builds the static musl
binary and attaches the archive, `.deb`, `.rpm`, AUR recipe, and checksums to a
draft release.

## 3. Read-only local checks

```bash
bash -n scripts/check-static-binary.sh
bash -n scripts/package-distribution.sh
bash -n scripts/smoke-test-distributions.sh
bash -n scripts/verify.sh

git diff --check
```

The distribution build requires `musl-gcc`, `readelf`, `dpkg-deb`, `rpmbuild`,
`sha256sum`, Docker, and the pinned Rust toolchain. GitHub Actions installs
these tools in its Ubuntu runner. You do not need to change your Arch machine's
system libraries to test the release binary.

## 4. Commit and open a pull request

```bash
git status --short
git add .cargo .github/workflows/release.yml \
  .github/workflows/distribution-ci.yml \
  scripts/check-static-binary.sh \
  scripts/package-distribution.sh \
  scripts/smoke-test-distributions.sh \
  packaging/aur docs README.md

git diff --cached --check
git diff --cached --stat
git commit -m "build(distribution): add portable x86_64 Linux packages"
git push -u origin distribution/portable-x86_64
```

Open a pull request targeting `main`. Wait for both the existing PRYN CI and
`PRYN Portable Distribution CI` to pass. Review the generated workflow
artifacts from the distribution CI run; do not publish yet.

## 5. Validate the musl executable on your machines

Download the CI artifact or build it on an Ubuntu build machine with the
pinned Rust toolchain, musl target, and musl-tools installed. Verify it with
`scripts/check-static-binary.sh`. Then test `--version`, `--help`, and
`check --format json` on:

- your Arch Linux laptop;
- your Kali Linux environment;
- your friend's Ubuntu machine;
- Alpine Linux (VM or container for startup; a real Alpine host is better).

For a host without BlueZ, NetworkManager, or an audio server, PRYN must start
and report those capabilities as unavailable/unknown. That is expected.

## 6. Merge and prepare v0.1.1

After the PR is merged, update `main`. Only then prepare the release commit:

1. Change `version = "0.1.0"` to `version = "0.1.1"` in `Cargo.toml`.
2. Run `cargo check` once; Cargo refreshes the `pryn` package entry in
   `Cargo.lock`.
3. Add a `0.1.1` section to `CHANGELOG.md` with the actual release date. Mention
   the static x86_64 musl archive, `.deb`, `.rpm`, AUR recipe, and distro smoke
   tests.
4. Commit the version/changelog/lockfile update and rerun
   `./scripts/verify.sh` plus `cargo publish --dry-run --locked`.

The release workflow rejects a tag whose version does not match Cargo.toml.

## 7. Cut and inspect v0.1.1

```bash
git switch main
git pull --ff-only origin main
git status --short
./scripts/verify.sh
cargo publish --dry-run --locked

git tag -a v0.1.1 -m "PRYN v0.1.1"
git push origin v0.1.1
```

The release workflow builds and validates the static binary and packages on
Ubuntu, Debian, Kali, Fedora, and Alpine containers. It creates a draft GitHub
Release with all assets. Review the SHA256SUMS, package contents, and generated
`pryn-bin.PKGBUILD`/`.SRCINFO` before publishing anything.

## 8. Publish packages

After all smoke tests and manual tests pass:

1. Review the draft GitHub Release, its checksum file, `.deb`, `.rpm`, static
   archive, and AUR metadata. Publish the GitHub Release so its archive URL is
   publicly downloadable.
2. On Arch, download `pryn-bin.PKGBUILD` from that published release, save it
   as `PKGBUILD`, and run `makepkg --printsrcinfo > .SRCINFO`,
   `makepkg --verifysource`, and `makepkg --cleanbuild --syncdeps`. Install the
   resulting package and test `pryn --version` and `pryn check`.
3. Only after the local Arch package test passes, submit `PKGBUILD` and
   `.SRCINFO` to the `pryn-bin` AUR repository.
4. Confirm `.deb` and `.rpm` installation tests passed in CI, and manually
   test their downloaded release assets if desired.
5. Publish `pryn 0.1.1` to crates.io after
   `cargo publish --dry-run --locked`.

The `.deb` and `.rpm` are initially direct-download packages, not signed
repositories. `apt install pryn` and `dnf install pryn` from configured package
indexes are deliberately deferred. Users install the local files with APT or
DNF. The RPM is unsigned and local installation therefore uses
`--nogpgcheck` until the signing milestone.

## Signing milestone

Use `docs/signing-and-adoption.md`. The chosen trigger is 500 combined public
downloads per month for three consecutive months, or three independent
downstream package-maintainer adoptions/requests, whichever happens first.
Do not add telemetry to count users.
