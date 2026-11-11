#!/bin/sh
# Builds the program and puts it, its menu entry and its icons under the given prefix (/app in the Flatpak).
#   sh linux/install.sh PREFIX
set -eu
PREFIX="${1:?install.sh needs the prefix to install under}"
ID=org.herbscontactsoptimizer.app
cargo build --release -p herbs-contacts-optimizer
install -Dm755 target/release/herbs-contacts-optimizer "$PREFIX/bin/herbs-contacts-optimizer"
install -Dm644 "linux/data/$ID.desktop" "$PREFIX/share/applications/$ID.desktop"
install -Dm644 "linux/data/$ID.metainfo.xml" "$PREFIX/share/metainfo/$ID.metainfo.xml"
for size in 256x256 512x512; do
    install -Dm644 "linux/icons/hicolor/$size/apps/$ID.png" "$PREFIX/share/icons/hicolor/$size/apps/$ID.png"
done
