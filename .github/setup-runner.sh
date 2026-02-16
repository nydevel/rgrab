#!/usr/bin/env bash
set -euo pipefail

# Setup script for self-hosted GitHub Actions runner with Docker
# Run as root on a clean Ubuntu/Debian server

RUNNER_USER="github-runner"
RUNNER_DIR="/home/$RUNNER_USER/actions-runner"
CACHE_DIR="/opt/rgrab-cache"

echo "=== 1. Install Docker ==="
if ! command -v docker &>/dev/null; then
    curl -fsSL https://get.docker.com | sh
fi

echo "=== 2. Create runner user ==="
if ! id "$RUNNER_USER" &>/dev/null; then
    useradd -m -s /bin/bash "$RUNNER_USER"
fi
usermod -aG docker "$RUNNER_USER"

echo "=== 3. Create cache directories ==="
mkdir -p "$CACHE_DIR/cargo" "$CACHE_DIR/target"
chown -R "$RUNNER_USER:$RUNNER_USER" "$CACHE_DIR"

echo "=== 4. Build builder image ==="
echo "Build the image and push to ghcr.io:"
echo ""
echo "  docker build -t ghcr.io/YOUR_USER/rgrab/builder:latest -f .github/Dockerfile.builder ."
echo "  docker push ghcr.io/YOUR_USER/rgrab/builder:latest"
echo ""
echo "Or build locally on this server:"
echo ""
echo "  cd /tmp && git clone https://github.com/YOUR_USER/rgrab.git"
echo "  docker build -t ghcr.io/YOUR_USER/rgrab/builder:latest -f rgrab/.github/Dockerfile.builder rgrab"
echo ""

echo "=== 5. Install GitHub Actions runner ==="
echo "Go to: GitHub repo -> Settings -> Actions -> Runners -> New self-hosted runner"
echo "Then run as $RUNNER_USER:"
echo ""
echo "  sudo -iu $RUNNER_USER"
echo "  mkdir -p $RUNNER_DIR && cd $RUNNER_DIR"
echo "  curl -o actions-runner-linux-x64.tar.gz -L https://github.com/actions/runner/releases/latest/download/actions-runner-linux-x64-2.322.0.tar.gz"
echo "  tar xzf actions-runner-linux-x64.tar.gz"
echo "  ./config.sh --url https://github.com/YOUR_USER/rgrab --token YOUR_TOKEN"
echo "  exit"
echo ""
echo "  # Install as service (as root):"
echo "  cd $RUNNER_DIR"
echo "  ./svc.sh install $RUNNER_USER"
echo "  ./svc.sh start"
echo ""

echo "=== Done ==="
echo "Replace YOUR_USER and YOUR_TOKEN with actual values."
echo "After setup, push a tag (git tag v0.1.0 && git push --tags) to trigger a build."
