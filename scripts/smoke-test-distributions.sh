#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'Usage: %s <version>\n' "$0" >&2
  exit 2
fi
version="$1"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$root/dist"
command -v docker >/dev/null 2>&1 || { echo 'ERROR: Docker is required for container smoke tests.' >&2; exit 1; }
test -s "$dist/pryn-v${version}-x86_64-unknown-linux-musl.tar.gz"
test -s "$dist/pryn_${version}_amd64.deb"
test -n "$(find "$dist" -maxdepth 1 -name "pryn-${version}-1.*.rpm" -print -quit)"

images=(
  "${PRYN_SMOKE_UBUNTU_IMAGE:-ubuntu:26.04}"
  "${PRYN_SMOKE_DEBIAN_IMAGE:-debian:13-slim}"
  "${PRYN_SMOKE_KALI_IMAGE:-kalilinux/kali-rolling}"
  "${PRYN_SMOKE_FEDORA_IMAGE:-fedora:latest}"
  "${PRYN_SMOKE_ALPINE_IMAGE:-alpine:latest}"
)

for image in "${images[@]}"; do
  echo "=== PRYN container smoke test: $image ==="
  docker pull "$image"
  docker run --rm \
    --env "PRYN_VERSION=$version" \
    --volume "$dist:/artifacts:ro" \
    --entrypoint /bin/sh \
    "$image" -ec '
      version="$PRYN_VERSION"
      if command -v apt-get >/dev/null 2>&1; then
        apt-get install -y --no-install-recommends "/artifacts/pryn_${version}_amd64.deb"
        bin=/usr/bin/pryn
      elif command -v dnf >/dev/null 2>&1; then
        dnf --disablerepo='*' install -y --nogpgcheck "/artifacts/pryn-${version}-1.x86_64.rpm"
        bin=/usr/bin/pryn
      else
        mkdir -p /tmp/pryn
        tar -xzf "/artifacts/pryn-v${version}-x86_64-unknown-linux-musl.tar.gz" -C /tmp/pryn
        bin="/tmp/pryn/pryn-v${version}/pryn"
      fi

      "$bin" --version | grep -F "pryn $version"
      "$bin" --help >/dev/null
      set +e
      "$bin" check --format json >/tmp/pryn-report.json
      status=$?
      set -e
      case "$status" in
        0|1) ;;
        *) echo "PRYN returned unexpected execution status: $status" >&2; exit 1 ;;
      esac
      grep -Eq "\"schema_version\"[[:space:]]*:|\"schemaVersion\"[[:space:]]*:" /tmp/pryn-report.json || {
        echo "PRYN did not emit the expected versioned JSON report" >&2
        exit 1
      }
      echo "PASS: installed PRYN executes and emits a versioned JSON report"
    '
done
