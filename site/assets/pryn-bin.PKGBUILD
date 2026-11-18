# Maintainer: MQSethar <hello at qasimio dot me>

pkgname=pryn-bin
pkgver=0.1.1
pkgrel=2
pkgdesc='Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio'
arch=('x86_64')
url='https://github.com/MQSethar/pryn'
license=('MIT OR Apache-2.0')
options=('!debug')
provides=("pryn=${pkgver}")
conflicts=('pryn')
depends=()
source=("pryn-${pkgver}.tar.gz::https://github.com/MQSethar/pryn/releases/download/v${pkgver}/pryn-v${pkgver}-${CARCH}-unknown-linux-musl.tar.gz")
sha256sums=('951c6c91094f70aa862deb9fd8bc36513fa7c9592ae1441ff334d3074789d0cf')

package() {
  install -Dm755 "$srcdir/pryn-v${pkgver}/pryn" \
    "$pkgdir/usr/bin/pryn"

  install -Dm644 "$srcdir/pryn-v${pkgver}/README.md" \
    "$pkgdir/usr/share/doc/${pkgname}/README.md"

  install -Dm644 "$srcdir/pryn-v${pkgver}/CHANGELOG.md" \
    "$pkgdir/usr/share/doc/${pkgname}/CHANGELOG.md"

  install -Dm644 "$srcdir/pryn-v${pkgver}/LICENSE-MIT" \
    "$pkgdir/usr/share/licenses/${pkgname}/LICENSE-MIT"

  install -Dm644 "$srcdir/pryn-v${pkgver}/LICENSE-APACHE" \
    "$pkgdir/usr/share/licenses/${pkgname}/LICENSE-APACHE"
}
