#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'Usage: %s <binary>
' "$0" >&2
  exit 2
fi

binary="$1"
if [[ ! -f "$binary" || ! -x "$binary" ]]; then
  printf 'ERROR: binary is missing or not executable: %s\n' "$binary" >&2
  exit 1
fi

command -v readelf >/dev/null 2>&1 || {
  echo 'ERROR: readelf is required (install binutils).' >&2
  exit 1
}

header="$(readelf -h "$binary")"
grep -Eq 'Machine:.*(Advanced Micro Devices X86-64|X86-64)' <<<"$header" || {
  echo 'ERROR: expected an x86_64 ELF executable.' >&2
  exit 1
}

program_headers="$(readelf -l "$binary")"
if grep -q 'INTERP' <<<"$program_headers"; then
  echo 'ERROR: binary has a PT_INTERP dynamic-loader dependency; it is not fully static.' >&2
  exit 1
fi

dynamic_entries="$(readelf -d "$binary" 2>/dev/null || true)"
if grep -q '(NEEDED)' <<<"$dynamic_entries"; then
  echo 'ERROR: binary has DT_NEEDED shared-library dependencies:' >&2
  grep '(NEEDED)' <<<"$dynamic_entries" >&2
  exit 1
fi

printf 'PASS: %s is x86_64 ELF with no interpreter or DT_NEEDED shared libraries.\n' "$binary"
file "$binary" || true
