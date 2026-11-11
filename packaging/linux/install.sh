#!/usr/bin/env bash
# Installs Floppy-Copy for the current user (no root needed): program, menu entry, icon.
#
#   ./install.sh                       from an extracted release archive
#   packaging/linux/install.sh         from the source tree, after `cargo build --release`
#   install.sh /path/to/hgf-gui       any binary
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

src="${1:-}"
if [ -z "$src" ]; then
  for c in "$here/floppy-copy" "$here/../../target/release/floppy-copy"; do
    if [ -x "$c" ]; then src="$c"; break; fi
  done
fi
if [ -z "$src" ] || [ ! -x "$src" ]; then
  echo "Program not found. Build it first (cargo build --release -p hgf-gui) or pass its path." >&2
  exit 1
fi
# Icons: every hicolor size plus the scalable SVG (release archive: ./icons, source tree: ../icons).
icondir="$here/icons"
[ -d "$icondir" ] || icondir="$here/../icons"
svg="$icondir/floppy-copy.svg"
[ -f "$svg" ] || svg="$here/../icon.svg"

bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor"
mkdir -p "$bin" "$apps"
install -m 755 "$src" "$bin/floppy-copy"
# Greaseweazle host tools shipped in the archive (Python + gw).
share="$HOME/.local/share/floppy-copy"
if [ -d "$here/greaseweazle" ]; then
  rm -rf "$share/greaseweazle"
  mkdir -p "$share"
  cp -a "$here/greaseweazle" "$share/greaseweazle"
  echo "Greaseweazle host tools installed in $share/greaseweazle."
fi
for png in "$icondir"/*.png; do
  size="$(basename "$png" .png)"
  mkdir -p "$icons/${size}x${size}/apps"
  install -m 644 "$png" "$icons/${size}x${size}/apps/floppy-copy.png"
done
if [ -f "$svg" ]; then
  mkdir -p "$icons/scalable/apps"
  install -m 644 "$svg" "$icons/scalable/apps/floppy-copy.svg"
fi
sed "s|^Exec=.*|Exec=$bin/floppy-copy|" "$here/floppy-copy.desktop" > "$apps/floppy-copy.desktop"
# Remove an install under the former name (HappyGreaseFloppy), so that the menu has one entry.
if [ -e "$apps/happygreasefloppy.desktop" ] || [ -e "$bin/happygreasefloppy" ]; then
  rm -f "$apps/happygreasefloppy.desktop" "$bin/happygreasefloppy"
  rm -f "$icons"/*/apps/happygreasefloppy.png "$icons"/scalable/apps/happygreasefloppy.svg
  echo "Removed the former HappyGreaseFloppy menu entry (settings are kept and carried over)."
fi
command -v update-desktop-database > /dev/null && update-desktop-database "$apps" || true
command -v gtk-update-icon-cache > /dev/null && gtk-update-icon-cache -q -t "$icons" || true
echo "Installed. Start Floppy-Copy from your applications menu (icon included)."
if ! command -v gw > /dev/null && [ ! -x "$share/greaseweazle/gw" ]; then
  echo "Note: the Greaseweazle tools (gw) were not found. Install them with: pip install --user greaseweazle"
fi
if ! groups | grep -qw dialout; then
  echo "Note: to access the Greaseweazle without root, add yourself to the dialout group:"
  echo "      sudo usermod -aG dialout \$USER   (then log out and back in)"
fi
