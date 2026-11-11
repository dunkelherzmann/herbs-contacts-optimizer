#!/bin/sh
# Builds the Linux app as a Flatpak. Run on Linux from anywhere in the repo.
#   --install   installs it for this user (flatpak run org.herbscontactsoptimizer.app)
# Needs Flathub's org.flatpak.Builder, the freedesktop 25.08 runtime and SDK and its Rust extension; what is
# missing is installed for this user on the first run. Ends with the app's --self-check.
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ID=org.herbscontactsoptimizer.app
# The app's version, for About (app/build.rs reads it; the Flatpak build has no git history).
sh "$ROOT/scripts/version.sh" > "$ROOT/version.txt"
cd "$ROOT/linux"
for ref in org.flatpak.Builder org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08; do
    flatpak info --user "$ref" >/dev/null 2>&1 || flatpak info "$ref" >/dev/null 2>&1 || flatpak install --user -y --noninteractive flathub "$ref"
done
STATE="$ROOT/linux/.flatpak-builder"
builder() { flatpak run org.flatpak.Builder --state-dir="$STATE" --disable-rofiles-fuse "$@"; }
if [ "${1:-}" = "--install" ]; then
    builder --user --install --force-clean build "$ID.yml"
    flatpak run --command=herbs-contacts-optimizer "$ID" --self-check
    printf 'Installed %s (%s)\n' "$ID" "$(cat "$ROOT/version.txt")"
else
    builder --force-clean build "$ID.yml"
    # `--run` takes no --disable-rofiles-fuse and its own exit status is unreliable, so the check is judged by its words.
    out="$(flatpak run org.flatpak.Builder --state-dir="$STATE" --run build "$ID.yml" herbs-contacts-optimizer --self-check 2>&1 || true)"
    printf '%s\n' "$out"
    printf '%s\n' "$out" | grep -q 'self-check: passed' || { echo "build.sh: the self-check failed" >&2; exit 1; }
    printf 'Built linux/build (%s)\n' "$(cat "$ROOT/version.txt")"
fi
