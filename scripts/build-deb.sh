#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/dist-linux"
IMAGE="comparew-deb:0.1.0"

export PATH="/opt/homebrew/bin:$PATH"
export DOCKER_HOST="unix://${HOME}/.colima/default/docker.sock"

if ! docker info >/dev/null 2>&1; then
  echo "Starting Colima (linux/amd64, for UOS)..."
  colima start --arch x86_64 --cpu 4 --memory 8 --disk 40
fi

mkdir -p "$OUT"

echo "Building self-contained CompareW .deb (Ubuntu 22.04 amd64, vendors WebKit 4.1)..."
docker build --network=host --platform linux/amd64 -f "$ROOT/Dockerfile.deb" -t "$IMAGE" "$ROOT"

cid="$(docker create "$IMAGE")"
docker cp "$cid":/out/. "$OUT/"
docker rm "$cid" >/dev/null

echo "Deb packages:"
ls -lh "$OUT"/*.deb
