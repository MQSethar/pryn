# Apply PRYN crate package hygiene patch

This small follow-up adds an explicit Cargo package allowlist and the missing homepage metadata. It keeps the committed Cargo.lock in the published CLI package while excluding maintainer-only files such as `.github/`, `.vscode/`, `APPLY.md`, `CONTRIBUTING.md`, `PATCH_NOTES.md`, `SECURITY.md`, and internal release documentation.

Apply from the clean `main` branch:

```bash
cd ~/Projects/pryn
git status --short
unzip -t ~/Downloads/pryn-crate-package-hygiene.zip
unzip -o ~/Downloads/pryn-crate-package-hygiene.zip -d .
cargo fmt --all -- --check
./scripts/verify.sh
cargo package --list
cargo publish --dry-run --locked
```

Review the package list. The intended files are the Cargo manifest/lock metadata, `src/**`, README, changelog, and both license texts. Cargo may also generate package metadata files such as `Cargo.toml.orig` and `.cargo_vcs_info.json`.

Do not run `cargo publish` until the crate name is confirmed available and the final tagged commit has passed CI.
