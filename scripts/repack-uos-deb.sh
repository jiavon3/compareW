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
if ! command -v patchelf >/dev/null 2>&1; then
  echo "patchelf is required to vendor glibc for UOS" >&2
  exit 1
fi

copy_lib() {
  local name="$1"
  local src
  for src in "/lib/x86_64-linux-gnu/$name" "/usr/lib/x86_64-linux-gnu/$name" "/lib64/$name"; do
    if [[ -e "$src" ]]; then
      cp -L "$src" "$LIBDIR/"
      return 0
    fi
  done
  return 1
}

copy_lib libstdc++.so.6 || true
copy_lib libgcc_s.so.1 || true

ld_src=""
for src in /lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2; do
  if [[ -e "$src" ]]; then
    ld_src="$src"
    break
  fi
done
if [[ -z "$ld_src" ]]; then
  echo "ld-linux-x86-64.so.2 not found" >&2
  exit 1
fi
cp -L "$ld_src" "$LIBDIR/ld-linux-x86-64.so.2"
chmod +x "$LIBDIR/ld-linux-x86-64.so.2"

for so in \
  libc.so.6 libm.so.6 libdl.so.2 libpthread.so.0 librt.so.1 \
  libresolv.so.2 libutil.so.1 libcrypt.so.1 \
  libnss_files.so.2 libnss_dns.so.2 libnss_compat.so.2 \
  libthread_db.so.1; do
  copy_lib "$so" || true
done

if [[ -d /usr/lib/x86_64-linux-gnu/gconv ]]; then
  mkdir -p "$LIBDIR/gconv"
  cp -a /usr/lib/x86_64-linux-gnu/gconv/. "$LIBDIR/gconv/" || true
fi

interp="/opt/comparew/usr/lib/ld-linux-x86-64.so.2"
while IFS= read -r -d '' elf; do
  if patchelf --print-interpreter "$elf" >/dev/null 2>&1; then
    patchelf --set-interpreter "$interp" "$elf"
  fi
done < <(find "$STAGE/opt/comparew" -type f -print0)

install -m 0755 "$ROOT/scripts/comparew-launch.sh" "$STAGE/usr/bin/comparew"

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
    -e '/^DBusActivatable=/d' \
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
if ! grep -q '^StartupNotify=' "$STAGE/usr/share/applications/comparew.desktop"; then
  printf '\nStartupNotify=true\n' >> "$STAGE/usr/share/applications/comparew.desktop"
fi
if ! grep -q '^Terminal=' "$STAGE/usr/share/applications/comparew.desktop"; then
  printf 'Terminal=false\n' >> "$STAGE/usr/share/applications/comparew.desktop"
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

# Ubuntu 22.04 dpkg-deb defaults to zstd; UOS V20 dpkg cannot read control.tar.zst.
dpkg-deb -Zgzip --root-owner-group -b "$STAGE" "$DEB_PATH"
if ar t "$DEB_PATH" | grep -q '\.zst$'; then
  echo "deb still contains zstd members; UOS dpkg cannot install it" >&2
  ar t "$DEB_PATH" >&2
  exit 1
fi
echo "Wrote $DEB_PATH"
ls -lh "$DEB_PATH"
