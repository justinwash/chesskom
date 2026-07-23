#!/usr/bin/env bash
# Build the Kobo binary (static ARM musl) and stage it for the device.
#
# Requires:
#   rustup target add armv7-unknown-linux-musleabihf
#   clang + lld on PATH (see .cargo/config.toml for the GNU-toolchain alternative)
#
# Output: dist/chesskom-kobo (a single self-contained ARM binary).
set -euo pipefail

TARGET=armv7-unknown-linux-musleabihf
cd "$(dirname "$0")/.."

echo "==> building chesskom-kobo for $TARGET (release)"
cargo build --release --target "$TARGET" -p chesskom-kobo

mkdir -p dist
cp "target/$TARGET/release/chesskom-kobo" dist/
echo "==> staged dist/chesskom-kobo"
file dist/chesskom-kobo || true
ls -lh dist/chesskom-kobo | awk '{print "    size:", $5}'

cat <<'EOF'

Next, on the device (requires FBInk installed):
  1. Copy dist/chesskom-kobo to the Kobo (USB mass storage, or scp if you have
     a terminal/SSH setup such as KoboRoot / kobo-usbms + dropbear).
  2. Make it executable:  chmod +x /mnt/onboard/.adds/chesskom/chesskom-kobo
  3. Run it:              /mnt/onboard/.adds/chesskom/chesskom-kobo
     (or wire it to a NickelMenu entry — see README)
EOF
