#!/usr/bin/env bash
# Package a release binary into dist/.
# Usage: package.sh <target-triple> <artifact-name> <version>
set -euo pipefail

target="$1"
artifact="$2"
version="$3"

bin="target/${target}/release/abstract"
mkdir -p dist

case "${target}" in
*-apple-darwin | *-linux-gnu) ;;

*-pc-windows-msvc)
    # Windows: standalone .exe (icon is embedded by build.rs) + zip with docs
    cp "${bin}.exe" "dist/abstract-${version}-${artifact}.exe"
    stage="dist/pkg"
    rm -rf "${stage}"
    mkdir -p "${stage}"
    cp "${bin}.exe" "${stage}/abstract.exe"
    cp README.md LICENSE "${stage}/"
    (cd "${stage}" && 7z a -tzip "../abstract-${version}-${artifact}.zip" . >/dev/null)
    rm -rf "${stage}"
    ls -lh dist/
    exit 0
    ;;

*)
    echo "unsupported target: ${target}" >&2
    exit 1
    ;;
esac

# Common tarball: binary + README + LICENSE
stage="dist/pkg"
rm -rf "${stage}"
mkdir -p "${stage}"
cp "${bin}" "${stage}/abstract"
cp README.md LICENSE "${stage}/"
tar -C "${stage}" -czf "dist/abstract-${version}-${artifact}.tar.gz" .
rm -rf "${stage}"

# Linux: AppImage built from an AppDir + appimagetool
if [[ "${target}" == *-linux-gnu ]]; then
    case "${target}" in
    x86_64-*) appimage_arch=x86_64 ;;
    aarch64-*) appimage_arch=aarch64 ;;
    *) echo "unsupported linux arch: ${target}" >&2; exit 1 ;;
    esac

    appdir="dist/AppDir"
    rm -rf "${appdir}"
    mkdir -p "${appdir}/usr/bin" \
        "${appdir}/usr/share/applications" \
        "${appdir}/usr/share/icons/hicolor/512x512/apps"
    cp "${bin}" "${appdir}/usr/bin/abstract"
    cp assets/abstract.desktop "${appdir}/usr/share/applications/abstract.desktop"
    cp assets/abstract.png "${appdir}/usr/share/icons/hicolor/512x512/apps/abstract.png"
    cp assets/abstract.desktop "${appdir}/abstract.desktop"
    cp assets/abstract.png "${appdir}/abstract.png"
    ln -s usr/bin/abstract "${appdir}/AppRun"
    ln -s abstract.png "${appdir}/.DirIcon"

    tool="dist/appimagetool"
    curl -fsSL -o "${tool}" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage"
    chmod +x "${tool}"
    ARCH="${appimage_arch}" "${tool}" --appimage-extract-and-run \
        "${appdir}" "dist/abstract-${version}-${artifact}.AppImage"
    rm -rf "${appdir}" "${tool}"
fi

# macOS: .app bundle, zipped and as a drag-to-Applications dmg.
# MACOS_SIGN_IDENTITY selects the codesign identity ("-" or unset = ad-hoc).
# MACOS_NOTARIZE=1 submits app + dmg to notarytool via the `abstract-notary`
# keychain profile and staples the tickets.
if [[ "${target}" == *-apple-darwin ]]; then
    app="dist/abstract.app"
    rm -rf "${app}"
    mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources"
    cp "${bin}" "${app}/Contents/MacOS/abstract"
    cp assets/abstract.icns "${app}/Contents/Resources/abstract.icns"
    cat >"${app}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>abstract</string>
    <key>CFBundleIdentifier</key>
    <string>com.fireflylabs.abstract</string>
    <key>CFBundleExecutable</key>
    <string>abstract</string>
    <key>CFBundleVersion</key>
    <string>${version}</string>
    <key>CFBundleShortVersionString</key>
    <string>${version}</string>
    <key>CFBundleIconFile</key>
    <string>abstract</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    identity="${MACOS_SIGN_IDENTITY:--}"
    if [[ "${identity}" == "-" ]]; then
        codesign --force --deep -s - "${app}"
    else
        codesign --force --deep --options runtime --timestamp \
            -s "${identity}" "${app}"
    fi

    if [[ "${MACOS_NOTARIZE:-0}" == "1" ]]; then
        notary_zip="dist/abstract-notary.zip"
        ditto -c -k --keepParent "${app}" "${notary_zip}"
        xcrun notarytool submit "${notary_zip}" \
            --keychain-profile abstract-notary --wait
        rm -f "${notary_zip}"
        xcrun stapler staple "${app}"
    fi

    ditto -c -k --keepParent "${app}" "dist/abstract-${version}-${artifact}.app.zip"

    dmgroot="dist/dmg"
    rm -rf "${dmgroot}"
    mkdir -p "${dmgroot}"
    cp -R "${app}" "${dmgroot}/"
    ln -s /Applications "${dmgroot}/Applications"
    dmg="dist/abstract-${version}-${artifact}.dmg"
    hdiutil create -volname "abstract" -srcfolder "${dmgroot}" -ov -format UDZO \
        "${dmg}" >/dev/null
    rm -rf "${dmgroot}" "${app}"

    if [[ "${identity}" != "-" ]]; then
        codesign --force --timestamp -s "${identity}" "${dmg}"
    fi
    if [[ "${MACOS_NOTARIZE:-0}" == "1" ]]; then
        xcrun notarytool submit "${dmg}" \
            --keychain-profile abstract-notary --wait
        xcrun stapler staple "${dmg}"
    fi
fi

ls -lh dist/
