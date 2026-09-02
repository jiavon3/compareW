#!/usr/bin/env bash
# Install the UOS .deb in Debian 10 (glibc 2.28, GLib 2.58 like UOS V20)
# and launch it under Xvfb with hostile host GTK/GIO/JSC env.
# Catches mixed GObject modules and invalid JSC_* options in CI.
# Does not reproduce kernel 4.19 (Docker uses the host kernel).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-$ROOT/dist-linux}"

DEB=""
if [[ -f "$SRC" && "$SRC" == *.deb ]]; then
  DEB="$SRC"
else
  while IFS= read -r -d '' candidate; do
    DEB="$candidate"
    break
  done < <(find "$SRC" -maxdepth 1 -name 'comparew_*.deb' -print0 2>/dev/null || true)
fi
if [[ -z "$DEB" || ! -f "$DEB" ]]; then
  echo "No comparew_*.deb in $SRC" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for Debian 10 smoke test" >&2
  exit 1
fi

DEB_ABS="$(cd "$(dirname "$DEB")" && pwd)/$(basename "$DEB")"
IMAGE="${COMPAREW_SMOKE_IMAGE:-debian:buster-slim}"

echo "Smoke-testing $DEB_ABS on $IMAGE (glibc 2.28 / GLib 2.58)"

docker pull "$IMAGE"
# -i is required or the heredoc never reaches the container and bash -s exits 0.
docker run --rm -i \
  -v "$DEB_ABS:/tmp/comparew.deb:ro" \
  "$IMAGE" \
  bash -s << 'INSIDE'
set -euo pipefail
echo "SMOKE_INSIDE_START"
export DEBIAN_FRONTEND=noninteractive
printf '%s\n' \
  'deb http://archive.debian.org/debian buster main' \
  > /etc/apt/sources.list
printf '%s\n' \
  'Acquire::Check-Valid-Until "false";' \
  'Acquire::AllowInsecureRepositories "true";' \
  > /etc/apt/apt.conf.d/99archive
apt-get update -o Acquire::Check-Valid-Until=false
# Do not install gvfs: archive.debian.org often cannot satisfy gvfs-daemons.
# libatk-adaptor + dconf still put host GTK/GIO modules on the search path.
apt-get install -y --no-install-recommends \
  xvfb xauth dbus-x11 fonts-dejavu-core \
  libgtk-3-0 libatk-adaptor at-spi2-core \
  ca-certificates
apt-get install -y --no-install-recommends dconf-gsettings-backend || true
dpkg -i /tmp/comparew.deb
test -x /usr/bin/comparew
test -x /opt/comparew/usr/bin/comparew
test -f /opt/comparew/usr/lib/libcomparew-gtk-redirect.so

# UOS DDE writes gtk-modules into /etc/gtk-3.0/settings.ini; GTK loads that last.
mkdir -p /etc/gtk-3.0
printf '%s\n' \
  '[Settings]' \
  'gtk-modules=gail:atk-bridge' \
  'gtk-im-module=ibus' \
  >/etc/gtk-3.0/settings.ini

export HOME=/tmp/comparew-home
export XDG_CACHE_HOME=/tmp/comparew-home/.cache
mkdir -p "$HOME" "$XDG_CACHE_HOME"

# Simulate UOS/DDE leaking host modules and an invalid JSC flag.
export GTK_MODULES=gail:atk-bridge
export GTK3_MODULES=atk-bridge
export GIO_EXTRA_MODULES=/usr/lib/x86_64-linux-gnu/gio/modules
export GTK_IM_MODULE=ibus
export XMODIFIERS=@im=ibus
export JSC_useWebAssembly=0
export GDK_BACKEND=wayland

set +e
xvfb-run -a -s '-screen 0 1280x720x24 -ac' \
  timeout --signal=TERM --kill-after=5 18 /usr/bin/comparew
status=$?
set -e

LOG="$XDG_CACHE_HOME/comparew/launch.log"
echo "launcher exit: $status"
if [[ -f "$LOG" ]]; then
  echo "---- launch.log ----"
  cat "$LOG"
  echo "---- end launch.log ----"
else
  echo "launch.log missing" >&2
  exit 1
fi

fail=0
while IFS= read -r pat; do
  if grep -E "$pat" "$LOG" >/dev/null; then
    echo "smoke fail: log matches /$pat/" >&2
    fail=1
  fi
done << 'PATS'
g_value_set_boxed
invalid option
No \.desktop files found
_dl_call_libc_early_init
EGL_NOT_INITIALIZED
failed to open swrast
权限不够
PATS

# 124 = timeout: process still running (success).
# 143 = SIGTERM from timeout (also still running).
# 139 = SIGSEGV.
if [[ "$status" -eq 139 || "$status" -eq 132 || "$status" -eq 126 || "$status" -eq 127 ]]; then
  echo "smoke fail: fatal exit $status" >&2
  fail=1
fi
if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
if [[ "$status" -eq 124 || "$status" -eq 143 || "$status" -eq 0 ]]; then
  echo "smoke ok (exit $status)"
  exit 0
fi
echo "smoke fail: unexpected exit $status" >&2
exit 1
INSIDE
