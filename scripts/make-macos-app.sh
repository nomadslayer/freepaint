#!/bin/sh
# Build "Free Paint.app" for macOS.
#   scripts/make-macos-app.sh                -> dist/Free Paint.app (native arch)
#   scripts/make-macos-app.sh --universal    -> universal (arm64 + x86_64) binary
#   scripts/make-macos-app.sh --install      -> also copy it to /Applications
set -eu
cd "$(dirname "$0")/.."

INSTALL=0
UNIVERSAL=0
for a in "$@"; do
    case "$a" in
        --install) INSTALL=1 ;;
        --universal) UNIVERSAL=1 ;;
    esac
done

if [ "$UNIVERSAL" = 1 ]; then
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    mkdir -p target/universal
    lipo -create -output target/universal/freepaint \
        target/aarch64-apple-darwin/release/freepaint \
        target/x86_64-apple-darwin/release/freepaint
    BIN=target/universal/freepaint
    ICONBIN=target/aarch64-apple-darwin/release/freepaint
else
    cargo build --release
    BIN=target/release/freepaint
    ICONBIN=$BIN
fi

APP="dist/Free Paint.app"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/freepaint"

# App icon: render every size, then pack it into an .icns
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
"$ICONBIN" --write-iconset "$WORK/AppIcon.iconset"
iconutil -c icns "$WORK/AppIcon.iconset" -o "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Free Paint</string>
    <key>CFBundleDisplayName</key>
    <string>Free Paint</string>
    <key>CFBundleIdentifier</key>
    <string>org.freepaint.FreePaint</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>freepaint</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.graphics-design</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP"
echo "Built $APP ($(lipo -archs "$APP/Contents/MacOS/freepaint" 2>/dev/null || echo native))"

if [ "$INSTALL" = 1 ]; then
    if [ -w /Applications ]; then DEST=/Applications; else DEST="$HOME/Applications"; mkdir -p "$DEST"; fi
    rm -rf "$DEST/Free Paint.app"
    cp -R "$APP" "$DEST/"
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DEST/Free Paint.app" >/dev/null 2>&1 || true
    echo "Installed $DEST/Free Paint.app"
fi
