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

# macOS: bare .app bundle (ad-hoc signed), zipped and as a drag-to-Applications dmg
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
    codesign --force --deep -s - "${app}"
    ditto -c -k --keepParent "${app}" "dist/abstract-${version}-${artifact}.app.zip"

    dmgroot="dist/dmg"
    rm -rf "${dmgroot}"
    mkdir -p "${dmgroot}"
    cp -R "${app}" "${dmgroot}/"
    ln -s /Applications "${dmgroot}/Applications"
    hdiutil create -volname "abstract" -srcfolder "${dmgroot}" -ov -format UDZO \
        "dist/abstract-${version}-${artifact}.dmg" >/dev/null
    rm -rf "${dmgroot}" "${app}"
fi

ls -lh dist/
