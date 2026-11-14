# Apply PRYN v0.1 release-preparation patch

This patch assumes the first-MVP and v0.1 follow-up source changes are already in your repository. It includes the local Wi-Fi helper fix, rustfmt-compatible formatting for `wifi.rs`, dual license files, Cargo package metadata, release packaging, and release documentation/workflow. It does not publish anything or create a Git tag.

## 1. Preserve your current work

```bash
cd ~/Projects/pryn
git status --short
tar --exclude='pryn/target' -czf ~/Projects/pryn-before-v0.1-release-prep.tar.gz -C ~/Projects pryn
```

Do not reset, clean, or discard your branch. The source overlay intentionally updates tracked project files.

## 2. Apply the archive

Download `pryn-v0.1-release-prep.zip` into `~/Downloads`, then:

```bash
cd ~/Projects/pryn
unzip -t ~/Downloads/pryn-v0.1-release-prep.zip
unzip -o ~/Downloads/pryn-v0.1-release-prep.zip -d .
```

## 3. Verify locally

```bash
cargo fmt --all
./scripts/verify.sh
cargo package --list
```

`./scripts/verify.sh` runs rustfmt check, strict Clippy, all unit/integration tests, a release build, and `cargo package --locked`. Do not commit unless it completes successfully.

## 4. Inspect the release diff

```bash
git diff --check
git status --short
git diff --stat
```

Review both license files and confirm all code in the repository may legally be distributed under the selected licenses. Read `docs/releasing.md` before creating a tag or publishing to crates.io.

## Important runtime status

The supplied latest output shows Wi-Fi and Bluetooth fully passing. Audio has unresolved connection/profile observations because the provider cannot confirm physical port availability and marks the currently active profiles unavailable. This contradiction remains `unknown`; it is not safe to convert it into `pass` or a hardware-failure finding. Validate sound output and microphone capture manually, and test standalone iwd on a host that actually uses iwd without NetworkManager.

The release workflow builds only x86_64 GNU/Linux. It creates a **draft** GitHub Release; it does not publish to crates.io, publish the GitHub release, or push a tag on your behalf.
