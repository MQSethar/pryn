# Apply the PRYN Wi-Fi / NetworkManager update

This update adds read-only NetworkManager D-Bus checks to the Wi-Fi subsystem. It does not change the public CLI: use `pryn check` and `pryn check wifi`.

## Before applying

From `~/Projects/pryn`, check your worktree:

```bash
git status --short
```

Keep your current branch and local changes. This archive intentionally does not include `src/main.rs`, `Cargo.lock`, or `rust-toolchain.toml`.

## Apply

Extract the archive into the repository root, replacing the listed files:

```bash
unzip -o ~/Downloads/pryn-wifi-networkmanager-update.zip -d .
```

This update uses `zbus` 5.19 with its blocking D-Bus API. Add it to the project and let Cargo update the existing lockfile:

```bash
cargo add zbus@5.19.0 --features blocking-api
cargo check
```

Commit both `Cargo.toml` and `Cargo.lock` with the implementation. Do not hand-edit the lockfile.

Format and verify:

```bash
cargo fmt --all
./scripts/verify.sh
```

Run the Wi-Fi checks:

```bash
cargo run -- check wifi
cargo run -- check --format json --output /tmp/pryn-wifi-report.json
python -m json.tool /tmp/pryn-wifi-report.json >/dev/null
```

If NetworkManager or the system D-Bus is unavailable, PRYN should report that the information could not be checked. It must not crash or claim success.

## What this update checks

- NetworkManager service availability on the system D-Bus.
- Global networking, Wi-Fi software-switch and hardware-radio state.
- Wi-Fi device state, including unmanaged, disconnected, unavailable, authentication, active, and failed states.
- NetworkManager firmware and device-plugin warning properties.
- NetworkManager's existing connectivity result, only when its connectivity-check service is configured and enabled. This is the last system-wide result, not a fresh Wi-Fi-specific test.

The connectivity result is system-wide and may reflect Ethernet or another interface. PRYN does not invoke NetworkManager's active connectivity test or send its own internet request.

Direct iwd inspection, IP address, route and DNS checks remain unimplemented in this update.
