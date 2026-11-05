# Apply the PRYN `check` command update

This update changes the user-facing command from `diagnose` to `check`, uses simpler terminal wording, and fixes the Clippy issues found on the Arch development machine. It does not add or remove dependencies.

From the PRYN repository root (`~/Projects/pryn`):

1. Make sure your work is committed or backed up: `git status`.
2. Extract this archive into the repository root, replacing files when prompted.
3. Remove the old lowercase duplicate README: `rm -f readme.md`.
4. Keep the existing `Cargo.lock` and `rust-toolchain.toml`.
5. Run `cargo fmt --all`.
6. Run `./scripts/verify.sh`.
7. Review the changes with `git diff`.

The supported user command is now `pryn check`. The old `pryn diagnose` command is intentionally not an alias.
