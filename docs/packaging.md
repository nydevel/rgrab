# Packaging and APT Repository

## Building the .deb Package

### Requirements

```bash
cargo install cargo-deb
```

### Building

```bash
# Full build (release + .deb)
cargo deb -p server

# Or step by step
cargo build --release -p server
cargo deb -p server --no-build
```

Result: `target/debian/rgrab_<version>-1_amd64.deb`

### Package Contents

```
/usr/bin/rgrab                         # server binary
/usr/bin/rgrab-tui                     # TUI client binary
/etc/rgrab/rgrab.toml                  # configuration (conffile, preserved on upgrade)
/usr/lib/systemd/system/rgrab.service  # systemd unit
```

On installation, the following happens automatically:
- System user `rgrab` is created (no shell)
- Directory `/var/lib/rgrab` is created for RocksDB data
- systemd service is registered (not started automatically)

## Installing from .deb File

```bash
sudo dpkg -i rgrab_0.1.0-1_amd64.deb
sudo apt-get install -f  # install missing dependencies if needed

sudo systemctl enable rgrab
sudo systemctl start rgrab
```

## Configuration

File `/etc/rgrab/rgrab.toml` (created automatically on installation):

```toml
data_dir = "/var/lib/rgrab"
listen = "0.0.0.0:3000"
log_level = "info"

[docker]
enabled = false
# socket = "/var/run/docker.sock"
#
# [[docker.containers]]
# name = "my-app"
# service = "my-app"
# environment = "production"
```

systemd starts the server with this config:
```
ExecStart=/usr/bin/rgrab --config /etc/rgrab/rgrab.toml
```

CLI arguments override values from the config:
```bash
# Override port at startup
rgrab --config /etc/rgrab/rgrab.toml --listen 0.0.0.0:8080
```

If Docker collector is enabled, add the `rgrab` user to the `docker` group:
```bash
sudo usermod -aG docker rgrab
```

After changing the config:
```bash
sudo systemctl restart rgrab
```

## GitHub Releases

The project uses GitHub Actions to automatically build and publish releases. To create a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers a workflow that builds the release binaries and .deb package, then publishes them as a GitHub Release.

### Installing from GitHub Releases

```bash
# Download the latest .deb
curl -LO https://github.com/nydevel/rgrab/releases/latest/download/rgrab_0.1.0-1_amd64.deb
sudo dpkg -i rgrab_0.1.0-1_amd64.deb
```

Or using the `gh` CLI:
```bash
gh release download --repo nydevel/rgrab --pattern '*.deb'
sudo dpkg -i rgrab_*.deb
```

## Setting Up an APT Repository

### Option 1: GitHub Releases (simplest)

Upload `.deb` to a GitHub Release. Users download and install directly (see above).

### Option 2: Self-hosted APT Repository

#### Generating the Repository

```bash
# Build .deb
cargo deb -p server

# Generate repo
./packaging/build-repo.sh ./apt-repo
```

#### GPG Signing (recommended)

```bash
# Create GPG key (once)
gpg --full-generate-key

# Export public key
gpg --armor --export rgrab > apt-repo/rgrab.gpg.key

# Rebuild repo (will sign it now)
./packaging/build-repo.sh ./apt-repo
```

#### Hosting

Place the contents of `apt-repo/` on any HTTP server:
- **GitHub Pages** -- free, simple
- **nginx** -- `location /apt { root /var/www; autoindex on; }`
- **S3/MinIO** -- for large-scale deployments

#### Client Setup

```bash
# Add GPG key
curl -fsSL https://repo.example.com/rgrab.gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/rgrab.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/rgrab.gpg] https://repo.example.com/ ./" | \
  sudo tee /etc/apt/sources.list.d/rgrab.list

# Install
sudo apt-get update
sudo apt-get install rgrab
```

Without GPG signing (not recommended for production):
```bash
echo "deb [trusted=yes] https://repo.example.com/ ./" | \
  sudo tee /etc/apt/sources.list.d/rgrab.list
sudo apt-get update
sudo apt-get install rgrab
```

## Service Management

```bash
# Start / stop
sudo systemctl start rgrab
sudo systemctl stop rgrab

# Enable auto-start
sudo systemctl enable rgrab

# View logs
sudo journalctl -u rgrab -f

# Status
sudo systemctl status rgrab
```

## Upgrading

```bash
sudo apt-get update
sudo apt-get upgrade rgrab
# or
sudo dpkg -i rgrab_<new_version>-1_amd64.deb
```

The config `/etc/rgrab/rgrab.toml` is preserved on upgrade (conffile).
Data in `/var/lib/rgrab` is not affected.

## Uninstalling

```bash
# Remove package (data is preserved)
sudo apt-get remove rgrab

# Full removal (including config)
sudo apt-get purge rgrab

# Remove data manually
sudo rm -rf /var/lib/rgrab
```

## Cross-compilation

```bash
# Install target
rustup target add aarch64-unknown-linux-gnu

# Build (requires a linker for the target platform)
cargo deb -p server --target aarch64-unknown-linux-gnu
```
