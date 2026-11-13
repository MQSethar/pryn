# Applying this PRYN source snapshot

This ZIP is a **complete project snapshot**, not a small overlay patch. Do not extract it directly over a working PRYN checkout until you have preserved your current work.

## Safe review

Create a separate directory and extract the archive there:

```bash
mkdir -p ~/Projects/pryn-audio-bluetooth-review
unzip -q ~/Downloads/pryn-audio-bluetooth-fixed.zip -d ~/Projects/pryn-audio-bluetooth-review
cd ~/Projects/pryn-audio-bluetooth-review
```

Review and validate the snapshot before replacing or merging anything into your main checkout. Keep your existing repository, branch, uncommitted changes, and backups untouched until the new snapshot passes verification.

## Verify

The project pins its Rust toolchain in `rust-toolchain.toml`. With that toolchain installed, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

Then exercise the read-only checks on the target Linux machine:

```bash
cargo run -- check audio
cargo run -- check bluetooth
cargo run -- check --format json
```

## Scope and safety

The update keeps PRYN read-only. Bluetooth pairing, connection, discovery, audio profile changes, audio playback, and microphone recording are not initiated. BlueZ state is read through the system D-Bus; audio state is read through the PulseAudio-compatible socket and, when available, native PipeWire `pw-dump` output. Real playback and capture still require a separate, explicit hardware test workflow.
