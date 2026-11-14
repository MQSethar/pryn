#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'Usage: %s <version-without-v> [target-triple]\n' "$0" >&2
  exit 2
fi

version="$1"
target="${2:-x86_64-unknown-linux-gnu}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][A-Za-z0-9.-]+)?$ ]]; then
  printf 'Invalid release version: %s\n' "$version" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$root/target/$target/release/pryn"
if [[ ! -x "$binary" ]]; then
  printf 'Missing release binary: %s\nBuild it first with cargo build --release --locked --target %s\n' "$binary" "$target" >&2
  exit 1
fi

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
package_dir="pryn-v$version"
mkdir -p "$stage/$package_dir" "$root/dist"
install -m 0755 "$binary" "$stage/$package_dir/pryn"
install -m 0644 "$root/README.md" "$stage/$package_dir/README.md"
install -m 0644 "$root/LICENSE-MIT" "$stage/$package_dir/LICENSE-MIT"
install -m 0644 "$root/LICENSE-APACHE" "$stage/$package_dir/LICENSE-APACHE"
install -m 0644 "$root/CHANGELOG.md" "$stage/$package_dir/CHANGELOG.md"

archive="$root/dist/pryn-v${version}-${target}.tar.gz"
tar -czf "$archive" -C "$stage" "$package_dir"
(
  cd "$root/dist"
  sha256sum "$(basename "$archive")" > SHA256SUMS
)
printf 'Created %s\n' "$archive"
printf 'Created %s\n' "$root/dist/SHA256SUMS"
