#!/bin/sh
# Builds "Herb's Contacts Optimizer.app" for this Mac into dist/: the program, its icon and its Info.plist, signed ad hoc.
# Uses only cargo and what ships with macOS (sips, iconutil, codesign).
#
#   macos/scripts/build.sh [--install]
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
VERSION="$(scripts/version.sh)"
APP="$ROOT/dist/Herb's Contacts Optimizer.app"

cargo build --release -p herbs-contacts-optimizer
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/herbs-contacts-optimizer "$APP/Contents/MacOS/herbs-contacts-optimizer"
sed "s/VERSION/$VERSION/g" macos/Info.plist >"$APP/Contents/Info.plist"

# The icon in every size macOS asks for, from the one large picture.
SET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$SET"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" app/assets/icon-1024.png --out "$SET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" app/assets/icon-1024.png --out "$SET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$(dirname "$SET")"

# Signed with no identity: enough for this Mac to run it; another Mac asks once (right-click, Open).
codesign --force --sign - "$APP" >/dev/null 2>&1

# --install puts it in /Applications, unless the app is running there.
if [ "${1:-}" = "--install" ]; then
    if pgrep -f "/Applications/Herb's Contacts Optimizer.app/" >/dev/null; then
        echo "Herb's Contacts Optimizer is running; quit it and run this again to install." >&2
        exit 1
    fi
    rm -rf "/Applications/Herb's Contacts Optimizer.app"
    cp -R "$APP" /Applications/
    echo "/Applications/Herb's Contacts Optimizer.app"
    exit 0
fi
echo "$APP"
