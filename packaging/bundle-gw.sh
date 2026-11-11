#!/usr/bin/env bash
# Builds the Greaseweazle host tools that Floppy-Copy ships with, so that nothing else has to be
# installed: a relocatable Python (python-build-standalone, installed by uv) with gw and its
# compiled data routines (needed for Apple II, Commodore 1541/1571 and Macintosh GCR disks).
#
#   packaging/bundle-gw.sh DIR [PYTHON] [SUBDIR]
#
#   DIR      folder to fill: DIR/gw (launcher) and DIR/SUBDIR (Python with gw installed)
#   PYTHON   uv Python request, default 3.12 (e.g. cpython-3.12-macos-x86_64-none for an Intel
#            Python on an Apple Silicon Mac, which needs Rosetta)
#   SUBDIR   default "python"; the macOS application holds python-arm64 and python-x86_64
#
# Windows (Git Bash): DIR also receives the official Windows release of gw (gw.exe, with the
# SPS/CAPS library for IPF images), used for every gw command; the Python serves the fast format
# detection. Downloading it uses the GitHub CLI (gh) with GH_TOKEN set.
#
# Environment: GREASEWEAZLE = host tools version installed with pip, a tag or a commit (default
# b209d70, the development version of April 2026: 1.23 plus `gw align`); GREASEWEAZLE_WIN = the
# official Windows release (default v1.23). Needs a C compiler (Xcode's command line tools, gcc,
# or Visual Studio's C++ build tools) and git.
set -euo pipefail

dir="${1:?usage: bundle-gw.sh DIR [PYTHON] [SUBDIR]}"
request="${2:-3.12}"
sub="${3:-python}"
tag="${GREASEWEAZLE:-b209d70}"
wintag="${GREASEWEAZLE_WIN:-v1.23}"

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) windows=1 ;;
  *) windows=0 ;;
esac

mkdir -p "$dir"
dir="$(cd "$dir" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if ! command -v uv >/dev/null 2>&1; then
  python3 -m pip install --quiet --user uv 2>/dev/null || python -m pip install --quiet --user uv
  export PATH="$HOME/.local/bin:$(python3 -m site --user-base 2>/dev/null || true)/bin:$PATH"
fi

echo "Python $request…"
UV_PYTHON_INSTALL_DIR="$work/pythons" uv python install "$request"
src="$(ls -d "$work/pythons"/cpython-* | head -n 1)"
rm -rf "${dir:?}/$sub"
cp -a "$src" "$dir/$sub"
if [ "$windows" = 1 ]; then
  py="$dir/$sub/python.exe"
else
  py="$dir/$sub/bin/python3"
fi
# pip refuses to install into an "externally managed" Python.
find "$dir/$sub" -name EXTERNALLY-MANAGED -delete
"$py" -m ensurepip --upgrade >/dev/null 2>&1 || true

echo "Greaseweazle host tools $tag…"
"$py" -m pip install --quiet --no-warn-script-location "git+https://github.com/keirf/greaseweazle@$tag"
# Without its compiled routines gw cannot handle GCR disks: refuse to ship such a bundle.
"$py" -c "import greaseweazle.optimised.optimised" || {
  echo "gw's compiled data routines were not built: is a C compiler installed?" >&2
  exit 1
}

# Parts of Python gw never uses.
for d in test idlelib tkinter turtledemo ensurepip lib2to3 pydoc_data; do
  find "$dir/$sub" -type d -path "*/lib*/python3*/$d" -prune -exec rm -rf {} + 2>/dev/null || true
  rm -rf "$dir/$sub/Lib/$d"
done
find "$dir/$sub" -name __pycache__ -type d -prune -exec rm -rf {} + 2>/dev/null || true
rm -rf "$dir/$sub/include" "$dir/$sub/share"

if [ "$windows" = 1 ]; then
  # gw.exe from the official release runs the gw commands (and brings the CAPS library).
  ver="${wintag#v}"
  gh release download "$wintag" -R keirf/greaseweazle -p "greaseweazle-${ver}*.zip" -D "$work/rel"
  unzip -q "$work"/rel/*.zip -d "$work/rel/x"
  inner="$(dirname "$(find "$work/rel/x" -iname gw.exe | head -n 1)")"
  cp -a "$inner"/. "$dir/"
else
  cat >"$dir/gw" <<'SH'
#!/bin/sh
# Greaseweazle host tools shipped with Floppy-Copy: run gw with the Python next to this file.
d=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
case "$(uname -m)" in arm64 | aarch64) a=arm64 ;; *) a=x86_64 ;; esac
for p in "$d/python-$a/bin/python3" "$d/python/bin/python3"; do
  if [ -x "$p" ]; then
    exec "$p" -c 'import sys; from greaseweazle.cli import main; sys.argv[0] = "gw"; sys.exit(main())' "$@"
  fi
done
echo "Floppy-Copy: no Python found in $d" >&2
exit 1
SH
  chmod +x "$dir/gw"
fi

"$dir/gw" --help >/dev/null 2>&1 || "$dir/gw.exe" --help >/dev/null 2>&1 || true
echo "Greaseweazle host tools $tag bundled in $dir"
