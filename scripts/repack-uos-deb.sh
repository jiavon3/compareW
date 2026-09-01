#!/usr/bin/env bash
# Pack Tauri's AppImage AppDir into a .deb that can install and run on UOS V20.
# V20 is Debian 10: no webkit2gtk 4.1, dpkg has no zstd, glibc 2.28.
# The binary is Ubuntu 22.04 + WebKit 4.1, so the package vendors that runtime
# and starts via the bundled ld-linux --library-path (never mix host libc 2.28).
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
  && [[ -e "$APPDIR/AppRun" ]]; then
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

if [[ ! -e "$APPDIR/AppRun" ]]; then
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

# Pull in whatever ldd sees on the build machine (Ubuntu 22.04), including glibc.
vendor_ldd() {
  local bin="$1"
  local src base
  [[ -e "$bin" ]] || return 0
  while IFS= read -r src; do
    [[ -z "$src" || ! -e "$src" ]] && continue
    case "$src" in
      /lib/*|/usr/lib/*|/lib64/*) ;;
      *) continue ;;
    esac
    base="$(basename "$src")"
    [[ -e "$LIBDIR/$base" ]] && continue
    cp -L "$src" "$LIBDIR/"
  done < <(ldd "$bin" 2>/dev/null | awk '/=> \// { print $3 }')
}

copy_lib libstdc++.so.6 || { echo "failed to vendor libstdc++.so.6" >&2; exit 1; }
copy_lib libgcc_s.so.1 || { echo "failed to vendor libgcc_s.so.1" >&2; exit 1; }

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

copy_lib libc.so.6 || { echo "failed to vendor libc.so.6" >&2; exit 1; }
for so in \
  libm.so.6 libdl.so.2 libpthread.so.0 librt.so.1 libanl.so.1 \
  libresolv.so.2 libutil.so.1 libcrypt.so.1 \
  libthread_db.so.1 libBrokenLocale.so.1; do
  copy_lib "$so" || true
done

# Host nsswitch.conf will dlopen libnss_*.so.2; those must be 2.35, not V20's 2.28.
for src in /lib/x86_64-linux-gnu/libnss_*.so* /usr/lib/x86_64-linux-gnu/libnss_*.so*; do
  [[ -e "$src" ]] || continue
  cp -L "$src" "$LIBDIR/" || true
done

if [[ -d /usr/lib/x86_64-linux-gnu/gconv ]]; then
  mkdir -p "$LIBDIR/gconv"
  cp -a /usr/lib/x86_64-linux-gnu/gconv/. "$LIBDIR/gconv/"
fi

# WebKit helpers + injected bundle (Tauri AppImage has missed these before).
if [[ -d /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1 ]]; then
  mkdir -p "$LIBDIR/x86_64-linux-gnu"
  rm -rf "$LIBDIR/x86_64-linux-gnu/webkit2gtk-4.1"
  cp -a /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1 "$LIBDIR/x86_64-linux-gnu/"
fi

# Tauri patches WebKit's /usr/lib prefix to ././lib; cwd will be APPDIR.
if [[ ! -e "$STAGE/opt/comparew/lib" ]]; then
  ln -sfn usr/lib "$STAGE/opt/comparew/lib"
elif [[ -d "$STAGE/opt/comparew/lib" && ! -e "$STAGE/opt/comparew/lib/x86_64-linux-gnu/webkit2gtk-4.1" \
    && -d "$LIBDIR/x86_64-linux-gnu/webkit2gtk-4.1" ]]; then
  mkdir -p "$STAGE/opt/comparew/lib/x86_64-linux-gnu"
  ln -sfn ../../usr/lib/x86_64-linux-gnu/webkit2gtk-4.1 \
    "$STAGE/opt/comparew/lib/x86_64-linux-gnu/webkit2gtk-4.1"
fi

MAIN=""
if [[ -e "$STAGE/opt/comparew/usr/bin/comparew" ]]; then
  MAIN="$STAGE/opt/comparew/usr/bin/comparew"
else
  echo "No usr/bin/comparew in AppDir" >&2
  exit 1
fi

vendor_ldd "$MAIN"
while IFS= read -r -d '' helper; do
  vendor_ldd "$helper"
done < <(find "$STAGE/opt/comparew" -type f \( \
  -name 'WebKitWebProcess' -o -name 'WebKitNetworkProcess' -o -name 'WebKitStorageProcess' \
  \) -print0 2>/dev/null || true)

interp="/opt/comparew/usr/lib/ld-linux-x86-64.so.2"
rpath="/opt/comparew/usr/lib:/opt/comparew/usr/lib/x86_64-linux-gnu"
while IFS= read -r -d '' elf; do
  base="$(basename "$elf")"
  case "$base" in
    ld-linux-x86-64.so.2|libc.so.6) continue ;;
  esac
  if patchelf --print-interpreter "$elf" >/dev/null 2>&1; then
    patchelf --set-interpreter "$interp" --force-rpath --set-rpath "$rpath" "$elf"
    chmod +x "$elf"
  elif patchelf --print-needed "$elf" >/dev/null 2>&1; then
    patchelf --force-rpath --set-rpath "$rpath" "$elf" 2>/dev/null || true
  fi
done < <(find "$STAGE/opt/comparew" -type f -print0)

find "$STAGE/opt/comparew" -maxdepth 1 -type f \( -name 'AppRun' -o -name 'AppRun.*' \) -exec chmod 0755 {} +
find "$STAGE/opt/comparew/usr/bin" -type f -exec chmod 0755 {} + 2>/dev/null || true
find "$STAGE/opt/comparew" -type f \( \
  -name 'WebKitWebProcess' -o -name 'WebKitNetworkProcess' -o -name 'WebKitStorageProcess' \
  \) -exec chmod 0755 {} + 2>/dev/null || true
chmod 0755 "$LIBDIR/ld-linux-x86-64.so.2"

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

DESKTOP_SRC="$(find "$STAGE/opt/comparew" -maxdepth 3 -name '*.desktop' | head -n1 || true)"
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

cat > "$STAGE/DEBIAN/postinst" << 'EOF'
#!/bin/sh
set -e
chmod 0755 /usr/bin/comparew /opt/comparew/AppRun /opt/comparew/usr/lib/ld-linux-x86-64.so.2 2>/dev/null || true
if [ -f /opt/comparew/AppRun.wrapped ]; then
  chmod 0755 /opt/comparew/AppRun.wrapped
fi
find /opt/comparew/usr/bin -type f -exec chmod 0755 {} + 2>/dev/null || true
find /opt/comparew -type f \( \
  -name 'WebKitWebProcess' -o -name 'WebKitNetworkProcess' -o -name 'WebKitStorageProcess' \
  \) -exec chmod 0755 {} + 2>/dev/null || true
exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postinst"

missing=""
for f in \
  "$STAGE/usr/bin/comparew" \
  "$STAGE/opt/comparew/AppRun" \
  "$LIBDIR/libc.so.6" \
  "$LIBDIR/ld-linux-x86-64.so.2" \
  "$LIBDIR/libstdc++.so.6" \
  "$LIBDIR/libgcc_s.so.1" \
  "$MAIN"; do
  if [[ ! -e "$f" ]]; then
    missing="$missing $f"
  fi
done
webkit_so=""
while IFS= read -r -d '' f; do
  webkit_so="$f"
  break
done < <(find "$STAGE/opt/comparew" -name 'libwebkit2gtk-4.1.so*' -print0 2>/dev/null || true)
if [[ -z "$webkit_so" ]]; then
  missing="$missing libwebkit2gtk-4.1.so"
fi
if [[ -n "$missing" ]]; then
  echo "UOS package is missing required files:$missing" >&2
  exit 1
fi
if [[ ! -x "$MAIN" || ! -x "$LIBDIR/ld-linux-x86-64.so.2" ]]; then
  echo "comparew binary or ld-linux is not executable" >&2
  exit 1
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
 Self-contained UOS build: vendors WebKitGTK 4.1 and glibc 2.35 so the
 app can install and run on UOS V20 (no libwebkit2gtk-4.1-0, glibc 2.28).
EOF

find "$STAGE/opt" "$STAGE/usr" -type d -exec chmod 0755 {} +
chmod 0755 "$STAGE/usr/bin/comparew" "$STAGE/opt/comparew/AppRun"
if [[ -e "$STAGE/opt/comparew/AppRun.wrapped" ]]; then
  chmod 0755 "$STAGE/opt/comparew/AppRun.wrapped"
fi
chmod 0755 "$STAGE/DEBIAN" "$STAGE/DEBIAN/postinst"
chmod 0644 "$STAGE/DEBIAN/control"

# Ubuntu 22.04 dpkg-deb defaults to zstd; UOS V20 dpkg cannot read control.tar.zst.
dpkg-deb -Zgzip --root-owner-group -b "$STAGE" "$DEB_PATH"
if ar t "$DEB_PATH" | grep -q '\.zst$'; then
  echo "deb still contains zstd members; UOS dpkg cannot install it" >&2
  ar t "$DEB_PATH" >&2
  exit 1
fi
if ! ar t "$DEB_PATH" | grep -q 'control.tar.gz'; then
  echo "expected control.tar.gz in deb, got:" >&2
  ar t "$DEB_PATH" >&2
  exit 1
fi
echo "Wrote $DEB_PATH"
ls -lh "$DEB_PATH"
