# PRYN Wi-Fi test fix

This patch updates two stale test expectations and clarifies the user-facing Wi-Fi connection wording.

Apply from the PRYN repository root:

```bash
git apply ~/Downloads/wifi-tests.patch
cargo fmt --all
./scripts/verify.sh
```

No dependencies, Cargo.lock, or toolchain files are changed.
