#!/usr/bin/env bash
# Pack Tauri's AppImage AppDir into a .deb that vendors WebKitGTK 4.1.
# UOS (and Debian 10/11) do not ship libwebkit2gtk-4.1-0, so the stock Tauri
# deb cannot be installed there. This package only depends on libc6.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT="${2:?--out requires a directory}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$OUT" ]]; then
  OUT="$ROOT/dist-linux"
fi
mkdir -p "$OUT"

VERSION="$(node -p "require('$ROOT/src-tauri/tauri.conf.json').version")"
PKG_NAME="comparew"
ARCH="amd64"
DEB_PATH="$OUT/${PKG_NAME}_${VERSION}_${ARCH}.deb"

find_one() {
  local path=""
  while IFS= read -r -d '' candidate; do
    path="$candidate"
    break
  done < <(find "$ROOT/src-tauri/target" "$@" -print0 2>/dev/null || true)
  if [[ -z "$path" ]]; then
    return 1
  fi
  printf '%s\n' "$path"
}

APPDIR=""
if APPDIR="$(find_one -type d -name '*.AppDir')" \
  && [[ -x "$APPDIR/AppRun" ]]; then
  echo "Using AppDir: $APPDIR"
else
  APPIMAGE="$(find_one -type f -name '*.AppImage')" || {
    echo "No AppDir or AppImage found under src-tauri/target" >&2
    exit 1
  }
  echo "Extracting AppImage: $APPIMAGE"
  chmod +x "$APPIMAGE"
  EXTRACT_DIR="$(mktemp -d)"
  trap 'rm -rf "$EXTRACT_DIR"' EXIT
  (cd "$EXTRACT_DIR" && APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGE" --appimage-extract)
  APPDIR="$EXTRACT_DIR/squashfs-root"
fi

if [[ ! -x "$APPDIR/AppRun" ]]; then
  echo "AppDir is missing AppRun: $APPDIR" >&2
  exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE" ${EXTRACT_DIR:-}' EXIT

mkdir -p \
  "$STAGE/opt/comparew" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/icons/hicolor/128x128/apps" \
  "$STAGE/DEBIAN"

cp -a "$APPDIR"/. "$STAGE/opt/comparew/"
chmod +x "$STAGE/opt/comparew/AppRun"

LIBDIR="$STAGE/opt/comparew/usr/lib"
mkdir -p "$LIBDIR"
for so in libstdc++.so.6 libgcc_s.so.1; do
  for src in \
    "/usr/lib/x86_64-linux-gnu/$so" \
    "/lib/x86_64-linux-gnu/$so"; do
    if [[ -e "$src" ]]; then
      cp -L "$src" "$LIBDIR/"
      break
    fi
  done
done

cat > "$STAGE/usr/bin/comparew" << 'EOF'
#!/bin/bash
set -euo pipefail
APPRUN="/opt/comparew/AppRun"
NEED_MAJOR=2
NEED_MINOR=34

have="$(ldd --version 2>/dev/null | awk 'NR==1 { print $NF; exit }' || true)"
have_major="${have%%.*}"
have_rest="${have#*.}"
have_minor="${have_rest%%.*}"

if [[ "$have_major" =~ ^[0-9]+$ && "$have_minor" =~ ^[0-9]+$ ]]; then
  if (( have_major < NEED_MAJOR || (have_major == NEED_MAJOR && have_minor < NEED_MINOR) )); then
    echo "CompareW 需要 glibc ${NEED_MAJOR}.${NEED_MINOR}+（Ubuntu 22.04 / Debian 12 / Deepin 23 或更新的统信 UOS）。" >&2
    echo "当前系统 glibc 为 ${have}，可以安装但无法运行。" >&2
    exit 1
  fi
fi

exec "$APPRUN" "$@"
EOF
chmod 0755 "$STAGE/usr/bin/comparew"

ICON_SRC=""
for candidate in \
  "$STAGE/opt/comparew/comparew.png" \
  "$STAGE/opt/comparew/CompareW.png" \
  "$STAGE/opt/comparew/.DirIcon"; do
  if [[ -f "$candidate" ]]; then
    ICON_SRC="$candidate"
    break
  fi
done
if [[ -z "$ICON_SRC" ]]; then
  ICON_SRC="$(find "$STAGE/opt/comparew" -type f \( -name '*.png' -o -name '*.svg' \) | head -n1 || true)"
fi
ICON_FIELD="comparew"
if [[ -n "$ICON_SRC" ]]; then
  cp -L "$ICON_SRC" "$STAGE/usr/share/icons/hicolor/128x128/apps/comparew.png"
  ICON_FIELD="/usr/share/icons/hicolor/128x128/apps/comparew.png"
fi

DESKTOP_SRC="$(find "$STAGE/opt/comparew" -name '*.desktop' | head -n1 || true)"
if [[ -n "$DESKTOP_SRC" ]]; then
  sed -E \
    -e 's|^Exec=.*|Exec=/usr/bin/comparew|' \
    -e "s|^Icon=.*|Icon=${ICON_FIELD}|" \
    "$DESKTOP_SRC" > "$STAGE/usr/share/applications/comparew.desktop"
else
  cat > "$STAGE/usr/share/applications/comparew.desktop" << EOF
[Desktop Entry]
Type=Application
Name=CompareW
Comment=轻量文本/文件夹比对
Exec=/usr/bin/comparew
Icon=${ICON_FIELD}
Terminal=false
Categories=Utility;
StartupNotify=true
EOF
fi

SIZE_KB="$(du -sk "$STAGE/opt" "$STAGE/usr" | awk '{ s += $1 } END { print s }')"

cat > "$STAGE/DEBIAN/control" << EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: CompareW <noreply@comparew>
Homepage: https://github.com/wangjiafeng93/compareW
Installed-Size: ${SIZE_KB}
Depends: libc6
Description: Lightweight two-pane text and folder comparison
 CompareW vendors WebKitGTK 4.1 so it can install on UOS and other
 Debian-based desktops that do not ship libwebkit2gtk-4.1-0.
EOF

find "$STAGE/opt" "$STAGE/usr" -type d -exec chmod 0755 {} +
chmod 0755 "$STAGE/usr/bin/comparew" "$STAGE/opt/comparew/AppRun"
chmod 0755 "$STAGE/DEBIAN"
chmod 0644 "$STAGE/DEBIAN/control"

dpkg-deb --root-owner-group -b "$STAGE" "$DEB_PATH"
echo "Wrote $DEB_PATH"
ls -lh "$DEB_PATH"
