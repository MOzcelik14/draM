#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [[ -z "${VERSION}" ]]; then
  VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
fi
VERSION="${VERSION#v}"

BIN_PATH="target/release/dram"
if [[ ! -f "${BIN_PATH}" ]]; then
  echo "error: ${BIN_PATH} not found. Run 'cargo build --release -p dram-app' first." >&2
  exit 1
fi

mkdir -p dist

# 1. Portable tar.gz
TAR_STAGE="$(mktemp -d)"
PKG_DIR="${TAR_STAGE}/draM-${VERSION}-linux-x86_64"
mkdir -p "${PKG_DIR}/locales"
cp "${BIN_PATH}" "${PKG_DIR}/dram"
cp -r locales/* "${PKG_DIR}/locales/"
cp README.md FORMAT.md "${PKG_DIR}/"
cp assets/logo-mark.svg "${PKG_DIR}/dram.svg"
tar -C "${TAR_STAGE}" -czf "dist/draM-${VERSION}-linux-x86_64.tar.gz" "draM-${VERSION}-linux-x86_64"
rm -rf "${TAR_STAGE}"

# 2. Debian .deb package
DEB_STAGE="$(mktemp -d)"
mkdir -p \
  "${DEB_STAGE}/DEBIAN" \
  "${DEB_STAGE}/usr/bin" \
  "${DEB_STAGE}/usr/share/applications" \
  "${DEB_STAGE}/usr/share/icons/hicolor/scalable/apps" \
  "${DEB_STAGE}/usr/share/doc/dram"

cp "${BIN_PATH}" "${DEB_STAGE}/usr/bin/dram"
chmod 0755 "${DEB_STAGE}/usr/bin/dram"
cp assets/logo-mark.svg "${DEB_STAGE}/usr/share/icons/hicolor/scalable/apps/dram.svg"
cp README.md FORMAT.md "${DEB_STAGE}/usr/share/doc/dram/"

cat > "${DEB_STAGE}/usr/share/applications/dram.desktop" <<'DESKTOP'
[Desktop Entry]
Name=draM.
Comment=Screenplay writing application
Exec=dram %f
Icon=dram
Terminal=false
Type=Application
MimeType=application/x-dram;
Categories=Office;WordProcessor;
DESKTOP

cat > "${DEB_STAGE}/DEBIAN/control" <<CONTROL
Package: dram
Version: ${VERSION}
Section: editors
Priority: optional
Architecture: amd64
Maintainer: draM. Contributors
Description: Desktop screenplay writing application in Rust
 Supports .dram, PDF, Fountain, Fade In, and Final Draft formats
 with first-class Turkish and English language support.
CONTROL

dpkg-deb --build --root-owner-group "${DEB_STAGE}" "dist/draM_${VERSION}_amd64.deb"
rm -rf "${DEB_STAGE}"

ls -lh dist/
