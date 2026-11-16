# Distribution release checklist

## Before tagging

- [ ] `Cargo.toml`, `Cargo.lock`, changelog, release tag, and release asset names use the same version.
- [ ] `./scripts/verify.sh` passes on the pinned toolchain.
- [ ] x86_64 musl release binary builds successfully.
- [ ] `scripts/check-static-binary.sh` confirms x86_64 ELF, no `PT_INTERP`, and no `DT_NEEDED` entries.
- [ ] Generic archive, `.deb`, `.rpm`, PKGBUILD, SRCINFO, and `SHA256SUMS` are generated.
- [ ] The PKGBUILD is locally validated with `makepkg --verifysource` and `makepkg --cleanbuild --syncdeps`.
- [ ] CI container smoke tests pass on Ubuntu, Debian, Kali, Fedora, and Alpine.
- [ ] Release assets are downloaded and independently checked against `SHA256SUMS`.
- [ ] Package contents contain the executable and intended documentation/license files.
- [ ] Packages declare no shared-library/runtime package dependencies.
- [ ] Release notes state x86_64-only, unsigned status, and AUR publication status accurately.
- [ ] No claim of physical distro validation is based only on container smoke tests.

## After publishing

- [ ] GitHub release is public and all assets are attached.
- [ ] crates.io shows the intended version, if a crate release is part of this version.
- [ ] docs.rs has successfully built the published crate.
- [ ] README and installation guide contain correct asset names and commands.
- [ ] AUR `pryn-bin` recipe is submitted only after the release archive is public and local makepkg validation passes.
- [ ] Verify the AUR package page exists before advertising `yay -S pryn-bin`.
- [ ] Update the support matrix only with tests actually completed.
