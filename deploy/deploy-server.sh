#!/usr/bin/env bash
# Deploy one immutable Voxtera Linux server release to the configured VPS.
set -euo pipefail

usage() {
  printf 'Usage: %s <release-tag> <ssh-key> <ssh-host> [ssh-user] [repo]\n' "$0" >&2
  exit 2
}

[[ $# -ge 3 && $# -le 5 ]] || usage
release_tag="$1"
ssh_key="$2"
ssh_host="$3"
ssh_user="${4:-ec2-user}"
repo="${5:-stoltembergg-png/voxtera}"
[[ "$release_tag" == v* ]] || { printf 'release tag must start with v\n' >&2; exit 2; }
[[ -f "$ssh_key" ]] || { printf 'SSH key file not found\n' >&2; exit 1; }
command -v gh >/dev/null || { printf 'gh CLI is required\n' >&2; exit 1; }
command -v ssh >/dev/null || { printf 'ssh is required\n' >&2; exit 1; }
command -v scp >/dev/null || { printf 'scp is required\n' >&2; exit 1; }

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
archive="voxtera-server-linux-x86_64-${release_tag}.tar.gz"
manifest="manifest-${release_tag}.json"

gh release download "$release_tag" --repo "$repo" \
  --pattern "$archive" --pattern "$manifest" --dir "$workdir"

python - "$workdir/$archive" "$workdir/$manifest" "$archive" <<'PY'
import hashlib
import json
import pathlib
import sys

archive = pathlib.Path(sys.argv[1])
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
expected_name = sys.argv[3]
record = manifest.get("artifacts", {}).get("linux-x86_64", {})
if record.get("archive") != expected_name:
    raise SystemExit("manifest archive does not match requested release")
expected_hash = record.get("sha256")
actual_hash = hashlib.sha256(archive.read_bytes()).hexdigest()
if not expected_hash or expected_hash != actual_hash:
    raise SystemExit("server archive SHA-256 does not match the release manifest")
print("release manifest and server archive verified")
PY

scp -q -i "$ssh_key" -o BatchMode=yes -o StrictHostKeyChecking=accept-new \
  "$workdir/$archive" "$ssh_user@$ssh_host:/tmp/$archive"

ssh -q -i "$ssh_key" -o BatchMode=yes -o StrictHostKeyChecking=accept-new \
  "$ssh_user@$ssh_host" "sudo bash -s -- '$release_tag' '$archive'" <<'REMOTE'
set -euo pipefail
release_tag="$1"
archive="$2"
base=/opt/voxtera
release_dir="$base/releases/$release_tag"
current_target=""
if [ -L "$base/current" ]; then
  current_target="$(readlink -f "$base/current")"
fi

sudo install -d -m 0755 "$base/releases"
if ! id voxtera >/dev/null 2>&1; then
  sudo useradd --system --home-dir /var/lib/voxtera --create-home --shell /sbin/nologin voxtera
fi
sudo install -d -o voxtera -g voxtera -m 0750 /var/lib/voxtera/userdata
sudo rm -rf "$release_dir"
sudo install -d -m 0755 "$release_dir"
sudo tar -xzf "/tmp/$archive" -C "$release_dir"
sudo rm -f "/tmp/$archive"
sudo test -x "$release_dir/veloren-server-cli"
sudo test -f "$release_dir/assets/common/canary.canary"
sudo test -f "$release_dir/voxtera-server.service"
sudo chown -R root:root "$release_dir"
sudo ln -sfn "releases/$release_tag" "$base/current"

# The server creates its default settings.ron on first start. Never overwrite
# an existing world/configuration during a release deployment.
sudo install -d -o voxtera -g voxtera -m 0750 \
  /var/lib/voxtera/userdata/server/server_config
sudo chown -R voxtera:voxtera /var/lib/voxtera/userdata
sudo install -o root -g root -m 0644 \
  "$release_dir/voxtera-server.service" /etc/systemd/system/voxtera-server.service
sudo systemctl daemon-reload
sudo systemctl enable voxtera-server.service
sudo systemctl restart voxtera-server.service

if ! sudo systemctl is-active --quiet voxtera-server.service; then
  if [ -n "$current_target" ] && [ -x "$current_target/veloren-server-cli" ]; then
    sudo ln -sfn "$current_target" "$base/current"
    sudo systemctl restart voxtera-server.service || true
  fi
  sudo systemctl status voxtera-server.service --no-pager || true
  exit 1
fi
sudo ss -ltn '( sport = :14004 or sport = :14005 or sport = :14006 )' || true
REMOTE

printf 'Voxtera server release deployed and service health verified.\n'
