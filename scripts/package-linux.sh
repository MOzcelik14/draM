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
  "${DEB_STAGE}/usr/share/metainfo" \
  "${DEB_STAGE}/usr/share/mime/packages" \
  "${DEB_STAGE}/usr/share/doc/dram"

cp "${BIN_PATH}" "${DEB_STAGE}/usr/bin/dram"
chmod 0755 "${DEB_STAGE}/usr/bin/dram"
cp assets/logo-mark.svg "${DEB_STAGE}/usr/share/icons/hicolor/scalable/apps/dram.svg"
cp assets/io.github.mozcelik14.dram.metainfo.xml "${DEB_STAGE}/usr/share/metainfo/"
cp assets/dram-mime.xml "${DEB_STAGE}/usr/share/mime/packages/"
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

# 3. RPM package (if rpmbuild is available)
if command -v rpmbuild >/dev/null 2>&1; then
  RPM_TOP="$(mktemp -d)"
  mkdir -p "${RPM_TOP}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
  cp "${BIN_PATH}" "${RPM_TOP}/SOURCES/dram"
  cp assets/logo-mark.svg "${RPM_TOP}/SOURCES/dram.svg"
  cp assets/io.github.mozcelik14.dram.metainfo.xml "${RPM_TOP}/SOURCES/"
  cp assets/dram-mime.xml "${RPM_TOP}/SOURCES/"
  cp README.md FORMAT.md "${RPM_TOP}/SOURCES/"

  cat > "${RPM_TOP}/SPECS/dram.spec" <<SPEC
%global debug_package %{nil}

Name:           dram
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Desktop screenplay writing application in Rust
License:        MIT OR Apache-2.0
URL:            https://github.com/MOzcelik14/draM

%description
Desktop screenplay writing application in Rust with Turkish and English
screenplay formatting, pagination, and PDF export.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_datadir}/metainfo
mkdir -p %{buildroot}%{_datadir}/mime/packages
mkdir -p %{buildroot}%{_datadir}/doc/dram

install -m 0755 "%{_sourcedir}/dram" %{buildroot}%{_bindir}/dram
install -m 0644 "%{_sourcedir}/dram.svg" %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/dram.svg
install -m 0644 "%{_sourcedir}/io.github.mozcelik14.dram.metainfo.xml" %{buildroot}%{_datadir}/metainfo/io.github.mozcelik14.dram.metainfo.xml
install -m 0644 "%{_sourcedir}/dram-mime.xml" %{buildroot}%{_datadir}/mime/packages/dram-mime.xml
install -m 0644 "%{_sourcedir}/README.md" %{buildroot}%{_datadir}/doc/dram/README.md
install -m 0644 "%{_sourcedir}/FORMAT.md" %{buildroot}%{_datadir}/doc/dram/FORMAT.md

cat > %{buildroot}%{_datadir}/applications/dram.desktop <<'DESKTOP'
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

%files
%{_bindir}/dram
%{_datadir}/applications/dram.desktop
%{_datadir}/icons/hicolor/scalable/apps/dram.svg
%{_datadir}/metainfo/io.github.mozcelik14.dram.metainfo.xml
%{_datadir}/mime/packages/dram-mime.xml
%{_datadir}/doc/dram/README.md
%{_datadir}/doc/dram/FORMAT.md
SPEC

  rpmbuild -bb --target x86_64 \
    --define "_topdir ${RPM_TOP}" \
    --define "_sourcedir ${RPM_TOP}/SOURCES" \
    "${RPM_TOP}/SPECS/dram.spec"
  find "${RPM_TOP}/RPMS" -name "*.rpm" -exec cp {} "dist/draM-${VERSION}-1.x86_64.rpm" \;
  rm -rf "${RPM_TOP}"
fi

# 4. AppImage (if appimagetool is available)
APPIMAGETOOL_BIN="${APPIMAGETOOL:-}"
if [[ -z "${APPIMAGETOOL_BIN}" ]] && command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL_BIN="$(command -v appimagetool)"
elif [[ -z "${APPIMAGETOOL_BIN}" && -x /tmp/appimagetool-x86_64.AppImage ]]; then
  APPIMAGETOOL_BIN="/tmp/appimagetool-x86_64.AppImage"
fi

if [[ -n "${APPIMAGETOOL_BIN}" ]]; then
  APPDIR="$(mktemp -d)/draM.AppDir"
  mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/share/applications" "${APPDIR}/usr/share/icons/hicolor/scalable/apps"
  cp "${BIN_PATH}" "${APPDIR}/usr/bin/dram"
  chmod 0755 "${APPDIR}/usr/bin/dram"
  cp assets/logo-mark.svg "${APPDIR}/dram.svg"
  cp assets/logo-mark.svg "${APPDIR}/usr/share/icons/hicolor/scalable/apps/dram.svg"

  cat > "${APPDIR}/dram.desktop" <<'DESKTOP'
[Desktop Entry]
Name=draM.
Comment=Screenplay writing application
Exec=dram %f
Icon=dram
Terminal=false
Type=Application
Categories=Office;WordProcessor;
DESKTOP
  cp "${APPDIR}/dram.desktop" "${APPDIR}/usr/share/applications/dram.desktop"

  cat > "${APPDIR}/AppRun" <<'APPRUN'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/usr/bin/dram" "$@"
APPRUN
  chmod 0755 "${APPDIR}/AppRun"

  ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "${APPIMAGETOOL_BIN}" "${APPDIR}" "dist/draM-${VERSION}-x86_64.AppImage"
  rm -rf "$(dirname "${APPDIR}")"
fi

ls -lh dist/
