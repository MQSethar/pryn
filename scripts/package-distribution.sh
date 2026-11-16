#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'Usage: %s <version> [musl-binary]\n' "$0" >&2
  exit 2
fi

version="$1"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][A-Za-z0-9.-]+)?$ ]]; then
  printf 'ERROR: invalid version: %s\n' "$version" >&2
  exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -n 1)"
if [[ "$manifest_version" != "$version" ]]; then
  printf 'ERROR: Cargo.toml version (%s) does not match release version (%s).\n' "$manifest_version" "$version" >&2
  exit 1
fi

target='x86_64-unknown-linux-musl'
binary="${2:-$root/target/$target/release/pryn}"
for tool in readelf dpkg-deb rpmbuild sha256sum tar gzip install; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'ERROR: required packaging tool is missing: %s\n' "$tool" >&2
    exit 1
  }
done

"$root/scripts/check-static-binary.sh" "$binary"

source_date_epoch="${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct 2>/dev/null || date +%s)}"
export SOURCE_DATE_EPOCH="$source_date_epoch"
maintainer="${PRYN_DEB_MAINTAINER:-PRYN Maintainers <102953455+MQSethar@users.noreply.github.com>}"

dist="$root/dist"
rm -rf "$dist"
mkdir -p "$dist"

# One universal x86_64 musl archive: this exact executable is reused by all packages.
stage="$(mktemp -d)"
rpm_top="$(mktemp -d)"
trap 'rm -rf "$stage" "$rpm_top"' EXIT
pkgdir="pryn-v$version"
mkdir -p "$stage/$pkgdir"
install -m 0755 "$binary" "$stage/$pkgdir/pryn"
install -m 0644 "$root/README.md" "$stage/$pkgdir/README.md"
install -m 0644 "$root/CHANGELOG.md" "$stage/$pkgdir/CHANGELOG.md"
install -m 0644 "$root/LICENSE-MIT" "$stage/$pkgdir/LICENSE-MIT"
install -m 0644 "$root/LICENSE-APACHE" "$stage/$pkgdir/LICENSE-APACHE"
find "$stage" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
archive="$dist/pryn-v${version}-${target}.tar.gz"
tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner -C "$stage" -cf - "$pkgdir" | gzip -n > "$archive"

# Debian-family package. Static musl executable: intentionally no Depends field.
debroot="$stage/deb-root"
mkdir -p "$debroot/DEBIAN" "$debroot/usr/bin" "$debroot/usr/share/doc/pryn" "$debroot/usr/share/doc/pryn"
install -m 0755 "$binary" "$debroot/usr/bin/pryn"
install -m 0644 "$root/README.md" "$debroot/usr/share/doc/pryn/README.md"
install -m 0644 "$root/CHANGELOG.md" "$debroot/usr/share/doc/pryn/CHANGELOG.md"
install -m 0644 "$root/LICENSE-MIT" "$debroot/usr/share/doc/pryn/LICENSE-MIT"
install -m 0644 "$root/LICENSE-APACHE" "$debroot/usr/share/doc/pryn/LICENSE-APACHE"
cat > "$debroot/DEBIAN/control" <<CONTROL
Package: pryn
Version: $version
Section: utils
Priority: optional
Architecture: amd64
Maintainer: $maintainer
Description: Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio
 PRYN inspects Linux network, Bluetooth, and audio state without changing
 system settings. Network and audio services are detected as optional host
 capabilities; they are not installed or managed by this package.
CONTROL
cat > "$debroot/usr/share/doc/pryn/copyright" <<COPYRIGHT
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: pryn
Source: https://github.com/MQSethar/pryn

Files: *
Copyright: 2026 PRYN contributors
License: MIT or Apache-2.0
 This package is dual-licensed. See LICENSE-MIT and LICENSE-APACHE installed
 in this directory for the complete license texts.
COPYRIGHT
find "$debroot" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
dpkg-deb --build --root-owner-group "$debroot" "$dist/pryn_${version}_amd64.deb" >/dev/null

# RPM-family package. AutoReqProv is disabled because the executable is fully static.
mkdir -p "$rpm_top/BUILD" "$rpm_top/BUILDROOT" "$rpm_top/RPMS" "$rpm_top/SOURCES" "$rpm_top/SPECS" "$rpm_top/SRPMS"
install -m 0755 "$binary" "$rpm_top/SOURCES/pryn"
install -m 0644 "$root/README.md" "$rpm_top/SOURCES/README.md"
install -m 0644 "$root/CHANGELOG.md" "$rpm_top/SOURCES/CHANGELOG.md"
install -m 0644 "$root/LICENSE-MIT" "$rpm_top/SOURCES/LICENSE-MIT"
install -m 0644 "$root/LICENSE-APACHE" "$rpm_top/SOURCES/LICENSE-APACHE"
rpm_changelog_date="$(date -u '+%a %b %d %Y')"
cat > "$rpm_top/SPECS/pryn.spec" <<SPEC
Name:           pryn
Version:        $version
Release:        1
Summary:        Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio
License:        MIT OR Apache-2.0
URL:            https://github.com/MQSethar/pryn
BuildArch:      x86_64
AutoReqProv:    no

%description
PRYN inspects Linux network, Bluetooth, and audio state without changing
system settings. Network and audio services are optional host capabilities.

%prep

%build

%install
install -D -m 0755 %{_sourcedir}/pryn %{buildroot}%{_bindir}/pryn
install -D -m 0644 %{_sourcedir}/README.md %{buildroot}%{_docdir}/%{name}/README.md
install -D -m 0644 %{_sourcedir}/CHANGELOG.md %{buildroot}%{_docdir}/%{name}/CHANGELOG.md
install -D -m 0644 %{_sourcedir}/LICENSE-MIT %{buildroot}%{_licensedir}/%{name}/LICENSE-MIT
install -D -m 0644 %{_sourcedir}/LICENSE-APACHE %{buildroot}%{_licensedir}/%{name}/LICENSE-APACHE

%files
%{_bindir}/pryn
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/CHANGELOG.md
%license %{_licensedir}/%{name}/LICENSE-MIT
%license %{_licensedir}/%{name}/LICENSE-APACHE

%changelog
* $rpm_changelog_date PRYN Maintainers <102953455+MQSethar@users.noreply.github.com> - $version-1
- Initial portable Linux package.
SPEC
rpmbuild -bb --define "_topdir $rpm_top" "$rpm_top/SPECS/pryn.spec" >/dev/null
rpmfile="$(find "$rpm_top/RPMS" -type f -name '*.rpm' -print -quit)"
test -n "$rpmfile"
install -m 0644 "$rpmfile" "$dist/$(basename "$rpmfile")"

# Generate an AUR binary package recipe pinned to this exact release archive.
archive_sha="$(sha256sum "$archive" | awk '{print $1}')"
cat > "$dist/pryn-bin.PKGBUILD" <<'PKGBUILD'
pkgname=pryn-bin
pkgver=VERSION_PLACEHOLDER
pkgrel=1
pkgdesc='Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio'
arch=('x86_64')
url='https://github.com/MQSethar/pryn'
license=('MIT' 'Apache-2.0')
provides=('pryn')
conflicts=('pryn')
depends=()
source=("pryn-${pkgver}.tar.gz::https://github.com/MQSethar/pryn/releases/download/v${pkgver}/pryn-v${pkgver}-x86_64-unknown-linux-musl.tar.gz")
sha256sums=('SHA256_PLACEHOLDER')

package() {
  install -Dm755 "$srcdir/pryn-v${pkgver}/pryn" "$pkgdir/usr/bin/pryn"
  install -Dm644 "$srcdir/pryn-v${pkgver}/README.md" "$pkgdir/usr/share/doc/pryn/README.md"
  install -Dm644 "$srcdir/pryn-v${pkgver}/CHANGELOG.md" "$pkgdir/usr/share/doc/pryn/CHANGELOG.md"
  install -Dm644 "$srcdir/pryn-v${pkgver}/LICENSE-MIT" "$pkgdir/usr/share/licenses/pryn/LICENSE-MIT"
  install -Dm644 "$srcdir/pryn-v${pkgver}/LICENSE-APACHE" "$pkgdir/usr/share/licenses/pryn/LICENSE-APACHE"
}
PKGBUILD
sed -i "s/VERSION_PLACEHOLDER/$version/; s/SHA256_PLACEHOLDER/$archive_sha/" "$dist/pryn-bin.PKGBUILD"
{
  printf 'pkgbase = pryn-bin\n'
  printf '\tpkgdesc = Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio\n'
  printf '\tpkgver = %s\n' "$version"
  printf '\tpkgrel = 1\n'
  printf '\turl = https://github.com/MQSethar/pryn\n'
  printf '\tarch = x86_64\n'
  printf '\tlicense = MIT\n'
  printf '\tlicense = Apache-2.0\n'
  printf '\tprovides = pryn\n'
  printf '\tconflicts = pryn\n'
  printf '\tsource = pryn-%s.tar.gz::https://github.com/MQSethar/pryn/releases/download/v%s/pryn-v%s-x86_64-unknown-linux-musl.tar.gz\n' "$version" "$version" "$version"
  printf '\tsha256sums = %s\n\n' "$archive_sha"
  printf 'pkgname = pryn-bin\n'
} > "$dist/pryn-bin.SRCINFO"

# Include checksums for every asset uploaded to the release.
(
  cd "$dist"
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\n' | LC_ALL=C sort | xargs -r sha256sum > SHA256SUMS
)

printf 'Distribution artifacts created in %s:\n' "$dist"
find "$dist" -maxdepth 1 -type f -printf '%f  %s bytes\n' | LC_ALL=C sort
