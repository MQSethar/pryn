# Distribution release checklist

- [ ] `x86_64-unknown-linux-musl` builds with the pinned Rust toolchain.
- [ ] `scripts/check-static-binary.sh` confirms x86_64 ELF, no PT_INTERP, and
      no DT_NEEDED entries.
- [ ] Universal archive installs and runs on Arch, Ubuntu, Kali, and Alpine.
- [ ] `.deb` installs and runs in Ubuntu, Debian, and Kali.
- [ ] `.rpm` installs and runs in Fedora.
- [ ] AUR PKGBUILD source hash matches the release archive.
- [ ] SHA256SUMS verifies all release assets.
- [ ] Package contents contain the executable, README/changelog, and license
      files only.
- [ ] Packages declare no shared-library/runtime package dependencies.
- [ ] Release notes clearly state x86_64-only and unsigned status.
- [ ] `v0.1.1` Cargo package version and Git tag match.
- [ ] `cargo publish --dry-run --locked` passes before crates.io publication.
