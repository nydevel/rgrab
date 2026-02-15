#!/bin/bash
set -euo pipefail

# Build APT repository from .deb files in target/debian/
# Usage: ./packaging/build-repo.sh [output_dir]
#
# Prerequisites: apt-get install -y dpkg-dev gpg
#
# This script:
# 1. Copies .deb files to the output directory
# 2. Generates Packages index
# 3. Signs with GPG (if key available)
# 4. Creates Release file

REPO_DIR="${1:-./apt-repo}"
DEB_DIR="./target/debian"

if [ ! -d "$DEB_DIR" ]; then
    echo "Error: $DEB_DIR not found. Run 'cargo deb -p server' first."
    exit 1
fi

mkdir -p "$REPO_DIR/pool"
cp "$DEB_DIR"/*.deb "$REPO_DIR/pool/"

cd "$REPO_DIR"

# Generate Packages index
dpkg-scanpackages pool /dev/null > Packages
gzip -k -f Packages

# Generate Release file
cat > Release <<EOF
Origin: rgrab
Label: rgrab
Suite: stable
Codename: stable
Architectures: amd64
Components: main
Description: rgrab APT repository
Date: $(date -Ru)
EOF

# Append checksums
apt-ftparchive release . >> Release 2>/dev/null || true

# Sign if GPG key is available
if gpg --list-secret-keys "rgrab" >/dev/null 2>&1; then
    gpg --armor --detach-sign --output Release.gpg Release
    gpg --armor --clearsign --output InRelease Release
    echo "Repository signed with GPG."
else
    echo "Warning: No GPG key 'rgrab' found. Repository is unsigned."
    echo "To sign: gpg --gen-key, then re-run this script."
fi

echo ""
echo "APT repository built in: $(pwd)"
echo "Contents:"
ls -lh pool/*.deb Packages* Release*
