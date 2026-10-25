#!/usr/bin/env bash
# Builds a release binary and wraps it in dist/Tattile.app.
set -euo pipefail

cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d '"' -f2)
APP=dist/Tattile.app

cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/tattile "$APP/Contents/MacOS/tattile"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Tattile</string>
  <key>CFBundleDisplayName</key><string>Tattile</string>
  <key>CFBundleIdentifier</key><string>com.tattile.app</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>tattile</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# An ad-hoc signature lets the app run on this Mac. Use a Developer ID to share it.
codesign --force --deep --sign - "$APP" >/dev/null
echo "Built $APP (version $VERSION)"
