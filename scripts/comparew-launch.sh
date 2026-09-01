#!/bin/bash
# Installed as /usr/bin/comparew. Keeps double-click failures visible on UOS.
set -u

APPDIR="/opt/comparew"
APPRUN="$APPDIR/AppRun"
LOGDIR="${XDG_CACHE_HOME:-$HOME/.cache}/comparew"
LOG="$LOGDIR/launch.log"

mkdir -p "$LOGDIR" || true
{
  echo "==== $(date '+%F %T') ===="
  echo "glibc: $(ldd --version 2>/dev/null | awk 'NR==1 { print $NF; exit }')"
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

if [[ ! -x "$APPRUN" ]]; then
  show_error "找不到 ${APPRUN}，请重新安装 CompareW。"
  exit 1
fi

# Host glibc on UOS V20 is too old for Ubuntu 22.04 WebKit; AppDir vendors its own.
# WebKit GPU/sandbox paths often abort with no window on DDE.
export WEBKIT_DISABLE_SANDBOX="${WEBKIT_DISABLE_SANDBOX:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
# Bundled glibc cannot use the host locale-archive.
export LC_ALL="${LC_ALL:-C.UTF-8}"
export LANG="${LANG:-C.UTF-8}"
if [[ -d "$APPDIR/usr/lib/gconv" ]]; then
  export GCONV_PATH="${GCONV_PATH:-$APPDIR/usr/lib/gconv}"
fi

set +e
"$APPRUN" "$@" >>"$LOG" 2>&1
status=$?

if [[ "$status" -ne 0 ]]; then
  show_error "启动失败（退出码 ${status}）。日志：${LOG}"
fi
exit "$status"
