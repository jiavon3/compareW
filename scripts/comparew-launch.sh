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

if [[ ! -x "$LDSO" || ! -e "$LIB/libc.so.6" ]]; then
  show_error "缺少打包的 glibc（${LDSO} 或 libc.so.6）。请重新安装 CompareW。"
  exit 1
fi
if [[ ! -x "$BIN" ]]; then
  show_error "找不到可执行文件 ${BIN}（权限不够时请 sudo chmod +x 该文件）。"
  exit 1
fi

# Host LD_PRELOAD / IM modules are built against glibc 2.28.
unset LD_PRELOAD
export APPDIR
export PATH="$APPDIR/usr/bin:${PATH:-/usr/bin}"
export XDG_DATA_DIRS="$APPDIR/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export WEBKIT_DISABLE_SANDBOX=1
export WEBKIT_FORCE_SANDBOX=0
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export LIBGL_ALWAYS_SOFTWARE=1
export NO_AT_BRIDGE=1
export GTK_A11Y=none
export GTK_IM_MODULE=gtk-im-context-simple
export LC_ALL=C.UTF-8
export LANG=C.UTF-8
if [[ -d "$LIB/gconv" ]]; then
  export GCONV_PATH="$LIB/gconv"
fi
if [[ -f "$LIB/gdk-pixbuf-2.0/2.10.0/loaders.cache" ]]; then
  export GDK_PIXBUF_MODULE_FILE="$LIB/gdk-pixbuf-2.0/2.10.0/loaders.cache"
fi
if [[ -d "$LIB/gio/modules" ]]; then
  export GIO_MODULE_DIR="$LIB/gio/modules"
fi

if [[ -d "$APPDIR/apprun-hooks" ]]; then
  for hook in "$APPDIR/apprun-hooks"/*.sh; do
    [[ -f "$hook" ]] || continue
    if grep -q '^[[:space:]]*exec ' "$hook" 2>/dev/null; then
      continue
    fi
    # shellcheck disable=SC1090
    . "$hook"
  done
  export GDK_BACKEND=x11
  export WEBKIT_DISABLE_SANDBOX=1
  export GTK_IM_MODULE=gtk-im-context-simple
fi

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

# WebKit rewrites helper prefixes to ././lib/... so cwd must be APPDIR.
cd "$APPDIR" || {
  show_error "无法进入 ${APPDIR}"
  exit 1
}

set +e
"$LDSO" --inhibit-cache --library-path "$libpath" "$BIN" "$@" >>"$LOG" 2>&1
status=$?

if [[ "$status" -ne 0 ]]; then
  show_error "启动失败（退出码 ${status}）。日志：${LOG}"
fi
exit "$status"
