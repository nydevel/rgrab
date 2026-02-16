#!/usr/bin/env bash
set -euo pipefail

# Usage: ./deploy.sh user@server
# Example: ./deploy.sh root@192.168.1.10

if [ $# -eq 0 ]; then
    echo "Usage: $0 user@host"
    exit 1
fi

TARGET="$1"

echo "=== Building release ==="
cargo build --release

echo "=== Building deb package ==="
cargo deb -p server --no-build

DEB=$(ls -t target/debian/*.deb | head -1)
echo "=== Deploying $DEB ==="

scp "$DEB" "$TARGET:/tmp/rgrab.deb"
ssh "$TARGET" 'dpkg -i /tmp/rgrab.deb && systemctl restart rgrab && rm /tmp/rgrab.deb'

echo "=== Done ==="
ssh "$TARGET" 'systemctl status rgrab --no-pager'
