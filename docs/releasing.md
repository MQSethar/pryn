# Releasing PRYN

This guide describes the release process after v0.1.1. The current distribution target is x86_64 Linux with one statically linked musl executable wrapped in native package formats. Do not create separate binaries for every distribution.

## Release gates

Before tagging:

1. Update the version in `Cargo.toml`, the root `Cargo.lock` package entry, and `CHANGELOG.md`.
2. Update `site/release.json` so the installation page points to the new release assets.
3. Update exact package names and installation commands in `README.md` and `docs/installation.md`.
4. Run `./scripts/verify.sh` and inspect `cargo package --list`.
5. Run the portable distribution CI workflow and ensure every package smoke test passes.
6. Check the PKGBUILD source URL and SHA-256 against the release archive.
7. Confirm real-machine compatibility claims are accurate and separate them from container tests.
8. Keep the working tree clean before creating the release tag.

## Cut a release

After the release commit is merged to `main`:

```bash
git switch main
git pull --ff-only origin main
git status --short
./scripts/verify.sh
cargo publish --dry-run --locked
```

If the crate source changed and a new crates.io version is intended, publish it once the dry run and package review pass:

```bash
cargo publish --locked
```

A crates.io version is effectively immutable after publication. Do not use a second publish attempt to correct documentation or packaging mistakes; fix those in a later patch release.

Create and push the matching tag:

```bash
git tag -a vX.Y.Z -m "PRYN vX.Y.Z"
git push origin vX.Y.Z
```

The tag-triggered release workflow verifies the manifest/tag version match, builds the x86_64 musl binary, checks that no interpreter/shared-library dependency exists, generates the archive/DEB/RPM/AUR metadata/checksums, and runs the distribution smoke tests. The workflow creates a draft release or attaches assets to an existing draft.

## Review and publish the GitHub Release

Before publishing the draft:

- Verify all checksums.
- Inspect the archive and native package contents.
- Confirm `.deb`, `.rpm`, and generic archive execute in the expected container matrix.
- Review the PKGBUILD and generated `.SRCINFO`.
- Check that all release notes, README links, and install commands match the new version.
- State that the RPM is unsigned until package signing is introduced.
- Do not claim AUR availability until the package has been submitted and its package page exists.

Then publish the GitHub Release. The AUR recipe source URL points to the GitHub release archive, so the archive must be publicly downloadable before the AUR submission.

## AUR update

For the first publication, create the `pryn-bin` package repository in the AUR account, then push a validated `PKGBUILD` and `.SRCINFO`. For later releases, update `pkgver`, `pkgrel`, source archive URL, and checksum; regenerate `.SRCINFO`; run `makepkg --verifysource` and `makepkg --cleanbuild --syncdeps`; then push the update.

Do not include an AUR command in user-facing documentation until the package is visible in the AUR index.

## Signing and hosted repositories

The current release uses GitHub HTTPS downloads and SHA-256 checksums. It does not include cryptographic release signatures or hosted APT/RPM repositories. See [signing-and-adoption.md](signing-and-adoption.md) for the adoption milestone and the work required before operating signing infrastructure.

## Website deployment

The static installation website lives in `site/`. `.github/workflows/pages.yml` publishes it from the repository's `main` branch using GitHub Pages. The repository owner must set **Settings → Pages → Build and deployment → Source → GitHub Actions** once. No JavaScript package build or third-party analytics service is required.
