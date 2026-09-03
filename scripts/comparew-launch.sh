#!/bin/bash
# Installed as /usr/bin/comparew.
# UOS V20 host glibc is 2.28. Never exec AppRun.wrapped with the host loader.
set -u

APPDIR="/opt/comparew"
LIB="$APPDIR/usr/lib"
LDSO="$LIB/ld-linux-x86-64.so.2"
# AppRun.wrapped is linuxdeploy's AppRun; /proc/self/exe becomes ld-linux and it
# errors "No .desktop files found". The real UI binary is usr/bin/comparew.
BIN="$APPDIR/usr/bin/comparew"
LOGDIR="${XDG_CACHE_HOME:-$HOME/.cache}/comparew"
LOG="$LOGDIR/launch.log"

mkdir -p "$LOGDIR" || true
{
  echo "==== $(date '+%F %T') ===="
  echo "host glibc: $(ldd --version 2>/dev/null | awk 'NR==1 { print $NF; exit }')"
  echo "ldso: $LDSO"
  echo "bin: $BIN"
  echo "args: $*"
} >>"$LOG" 2>&1 || true

show_error() {
  local msg="$1"
  echo "$msg" >>"$LOG" 2>&1 || true
  if command -v zenity >/dev/null 2>&1; then
    zenity --error --no-wrap --title="CompareW" --text="$msg" 2>/dev/null && return
  fi
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$msg" << 'PY' 2>/dev/null && return
import sys
text = sys.argv[1]
try:
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk
    d = Gtk.MessageDialog(
        message_type=Gtk.MessageType.ERROR,
        buttons=Gtk.ButtonsType.OK,
        text="CompareW",
    )
    d.format_secondary_text(text)
    d.run()
    d.destroy()
except Exception:
    sys.exit(1)
PY
  fi
  if command -v notify-send >/dev/null 2>&1; then
    notify-send -u critical "CompareW" "$msg" 2>/dev/null || true
  fi
}

# Host GTK/GIO/pixbuf/IM modules are GLib 2.58; bundled GTK is 3.24/GLib 2.72.
# Any fallback to /usr will g_value_set_boxed and SIGSEGV (exit 139).
# JSC treats unknown JSC_* env vars as fatal "invalid option".
apply_isolation_env() {
  unset LD_PRELOAD GTK_MODULES GTK3_MODULES GIO_EXTRA_MODULES GTK_PATH
  unset XMODIFIERS QT_IM_MODULE GI_TYPELIB_PATH
  unset WEBKIT_FORCE_SANDBOX
  export GTK_MODULES=""
  export GTK3_MODULES=""
  export GIO_EXTRA_MODULES=""
  export GSETTINGS_BACKEND=memory
  export GIO_USE_VFS=local
  export GTK_THEME=Adwaita
  export GTK_A11Y=none
  export NO_AT_BRIDGE=1
  export GTK_IM_MODULE=gtk-im-context-simple
  export GTK_EXE_PREFIX="$APPDIR/usr"
  export GTK_DATA_PREFIX="$APPDIR/usr"
  export GTK_PATH="$LIB/gtk-3.0"
  export GIO_MODULE_DIR="$LIB/gio/modules"
  export GTK_IM_MODULE_FILE="$LIB/gtk-3.0/3.0.0/immodules.cache"
  export GSETTINGS_SCHEMA_DIR="$APPDIR/usr/share/glib-2.0/schemas"
  export GDK_BACKEND=x11
  export XDG_CURRENT_DESKTOP=GNOME
  unset DESKTOP_SESSION GNOME_DESKTOP_SESSION_ID DEEPIN_SESSION_TYPE
  export APPDIR
  export PATH="$APPDIR/usr/bin:${PATH:-/usr/bin}"
  export XDG_DATA_DIRS="$APPDIR/usr/share"
  # GTK always reads /etc/gtk-3.0/settings.ini last; DDE puts gtk-modules there.
  export XDG_CONFIG_HOME="$LOGDIR/xdg-config"
  export XDG_CONFIG_DIRS="$APPDIR/usr/etc"
  mkdir -p "$XDG_CONFIG_HOME/gtk-3.0" 2>/dev/null || true
  printf '%s\n' \
    '[Settings]' \
    'gtk-modules=' \
    'gtk-im-module=gtk-im-context-simple' \
    >"$XDG_CONFIG_HOME/gtk-3.0/settings.ini" 2>/dev/null || true
  # Never export LD_PRELOAD here: the .so needs glibc 2.34 and would break
  # host env/grep/find (glibc 2.28), including the JSC_* cleanup below.

  export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
  export WEBKIT_DISABLE_COMPOSITING_MODE=1
  export WEBKIT_DISABLE_DMABUF_RENDERER=1

  export LIBGL_ALWAYS_SOFTWARE=1
  export GALLIUM_DRIVER=softpipe
  export LIBGL_DRIVERS_PATH="$LIB/dri"
  export __EGL_VENDOR_LIBRARY_DIRS="$APPDIR/usr/share/glvnd/egl_vendor.d"
  if [[ -f "$APPDIR/usr/share/glvnd/egl_vendor.d/50_mesa.json" ]]; then
    export __EGL_VENDOR_LIBRARY_FILENAMES="$APPDIR/usr/share/glvnd/egl_vendor.d/50_mesa.json"
  fi
  # Host Vulkan ICDs / GStreamer / Enchant plugins are also ABI-mixed.
  export VK_ICD_FILENAMES="/nonexistent-comparew-vulkan.json"
  export VK_DRIVER_FILES="/nonexistent-comparew-vulkan.json"
  export GST_PLUGIN_SYSTEM_PATH_1_0="/nonexistent-comparew-gstreamer"
  export GST_PLUGIN_SYSTEM_PATH="/nonexistent-comparew-gstreamer"
  export GST_PLUGIN_PATH="/nonexistent-comparew-gstreamer"
  export ENCHANT_MODULE_DIR="/nonexistent-comparew-enchant"

  local pixbuf=""
  local d
  for d in \
    "$LIB/gdk-pixbuf-2.0/2.10.0" \
    "$LIB/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0"; do
    if [[ -f "$d/loaders.cache" || -d "$d/loaders" ]]; then
      pixbuf="$d"
      break
    fi
  done
  if [[ -z "$pixbuf" ]]; then
    pixbuf="$LIB/gdk-pixbuf-2.0/2.10.0"
  fi
  export GDK_PIXBUF_MODULE_FILE="$pixbuf/loaders.cache"
  export GDK_PIXBUF_MODULEDIR="$pixbuf/loaders"

  if [[ -d "$LIB/girepository-1.0" ]]; then
    export GI_TYPELIB_PATH="$LIB/girepository-1.0"
  fi
  local helper=""
  for d in "$LIB/webkit2gtk-4.1" "$LIB/x86_64-linux-gnu/webkit2gtk-4.1"; do
    if [[ -x "$d/WebKitWebProcess" ]]; then
      helper="$d"
      break
    fi
  done
  if [[ -n "$helper" ]]; then
    export WEBKIT_EXEC_PATH="$helper"
  fi
  if [[ -d "$LIB/gconv" ]]; then
    export GCONV_PATH="$LIB/gconv"
  fi
  local icu=""
  icu="$(find "$APPDIR/usr/share/icu" -name 'icudt*.dat' 2>/dev/null | head -n1 || true)"
  if [[ -n "$icu" ]]; then
    export ICU_DATA="$(dirname "$icu")"
  fi

  local name
  while IFS= read -r name; do
    [[ -z "$name" || "$name" == JSC_useJIT ]] && continue
    [[ "$name" == JSC_* ]] || continue
    unset "$name"
  done < <(compgen -e)
  unset JSC_useWebAssembly
  export JSC_useJIT=0
}

if [[ ! -x "$LDSO" || ! -e "$LIB/libc.so.6" ]]; then
  show_error "缺少打包的 glibc（${LDSO} 或 libc.so.6）。请重新安装 CompareW。"
  exit 1
fi
if [[ ! -x "$BIN" ]]; then
  show_error "找不到可执行文件 ${BIN}（权限不够时请 sudo chmod +x 该文件）。"
  exit 1
fi

export LC_ALL=C.UTF-8
export LANG=C.UTF-8
apply_isolation_env

if [[ -d "$APPDIR/apprun-hooks" ]]; then
  for hook in "$APPDIR/apprun-hooks"/*.sh; do
    [[ -f "$hook" ]] || continue
    if grep -q '^[[:space:]]*exec ' "$hook" 2>/dev/null; then
      continue
    fi
    # shellcheck disable=SC1090
    . "$hook"
  done
  apply_isolation_env
fi

jsc_log=""
while IFS= read -r name; do
  [[ "$name" == JSC_* ]] || continue
  jsc_log="${jsc_log}${name}=${!name} "
done < <(compgen -e)
{
  echo "XDG_CONFIG_DIRS=$XDG_CONFIG_DIRS"
  echo "COMPAREW_GTK_REDIRECT=$LIB/libcomparew-gtk-redirect.so"
  echo "GIO_MODULE_DIR=$GIO_MODULE_DIR"
  echo "GTK_PATH=$GTK_PATH"
  echo "GTK_EXE_PREFIX=$GTK_EXE_PREFIX"
  echo "GTK_IM_MODULE_FILE=$GTK_IM_MODULE_FILE"
  echo "GDK_PIXBUF_MODULE_FILE=$GDK_PIXBUF_MODULE_FILE"
  echo "GDK_BACKEND=$GDK_BACKEND"
  echo "WEBKIT_EXEC_PATH=${WEBKIT_EXEC_PATH:-}"
  echo "LIBGL_DRIVERS_PATH=$LIBGL_DRIVERS_PATH"
  echo "JSC=$jsc_log"
} >>"$LOG" 2>&1 || true

libpath=""
while IFS= read -r d; do
  [[ -z "$d" ]] && continue
  case ":$libpath:" in
    *":$d:"*) ;;
    *) libpath="${libpath:+$libpath:}$d" ;;
  esac
done < <(find "$APPDIR/usr" "$APPDIR/lib" -name '*.so*' -printf '%h\n' 2>/dev/null | sort -u)
if [[ -z "$libpath" ]]; then
  libpath="$LIB"
fi
export LD_LIBRARY_PATH="$libpath"

# WebKit rewrites helper prefixes to ././lib/... so cwd must be APPDIR.
cd "$APPDIR" || {
  show_error "无法进入 ${APPDIR}"
  exit 1
}

set +e
# Do not invoke ld-linux as a program (--library-path). On V20 that SIGSEGVs.
# usr/bin/comparew is already patchelf'd to the bundled interpreter + RPATH.
# Preload only this process (and children). Host zenity/python stay on glibc 2.28.
redirect_so="$LIB/libcomparew-gtk-redirect.so"
if [[ -f "$redirect_so" ]]; then
  LD_PRELOAD="$redirect_so" "$BIN" "$@" >>"$LOG" 2>&1
else
  "$BIN" "$@" >>"$LOG" 2>&1
fi
status=$?

if [[ "$status" -ne 0 ]]; then
  show_error "启动失败（退出码 ${status}）。日志：${LOG}"
fi
exit "$status"
