#!/bin/bash

# 🚀 One-Click Installation Script for Rust Edge Compute Framework
# Bare Metal Deployment
# Usage: sudo ./scripts/install-bare-metal.sh --binary ./rust-edge-compute --config config/production.toml

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Ensure running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}❌ Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Default values
BINARY_PATH=""
CONFIG_PATH=""
USER_NAME="edge-compute"
USER_ID=9999
GROUP_NAME="edge-compute"
GROUP_ID=9999
INSTALL_DIR="/opt/edge-compute"
VERSION=$(date +%Y%m%d)
DATA_DIR="/opt/edge-compute/data"
LOG_DIR="/var/log/edge-compute"
CONFIG_DIR="/opt/edge-compute/etc"
SYSTEMD_DIR="/etc/systemd/system"

usage() {
    cat <<EOF
Usage: sudo $0 [OPTIONS]

Options:
    --binary <PATH>         Path to binary (required)
    --config <PATH>         Config file path (optional)
    --version <VERSION>     Version number (default: timestamp)
    --install-dir <DIR>     Installation directory (default: /opt/edge-compute)
    --user <NAME>          Service user name (default: edge-compute)
    --group <NAME>         Service group name (default: edge-compute)
    --help                 Show this help message

Examples:
    sudo ./scripts/install-bare-metal.sh --binary ./rust-edge-compute
    sudo ./scripts/install-bare-metal.sh --binary ./rust-edge-compute --config config/production.toml --version 0.1.0
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        --config)
            CONFIG_PATH="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --user)
            USER_NAME="$2"
            shift 2
            ;;
        --group)
            GROUP_NAME="$2"
            shift 2
            ;;
        --help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Validate inputs
if [ -z "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Error: --binary is required${NC}"
    exit 1
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Error: Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

# Display installation plan
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🚀 Rust Edge Compute Framework - Bare Metal Installation${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}📋 Installation Plan:${NC}"
echo "  Binary:         $BINARY_PATH"
echo "  Version:        $VERSION"
echo "  Install Dir:    $INSTALL_DIR"
echo "  User:           $USER_NAME:$GROUP_NAME"
echo "  Data Dir:       $DATA_DIR"
echo "  Log Dir:        $LOG_DIR"
echo "  Config Dir:     $CONFIG_DIR"
echo ""

# Step 1: Check system dependencies
echo -e "${YELLOW}📦 Step 1: Checking system dependencies...${NC}"
MISSING_DEPS=""
for cmd in curl jq; do
    if ! command -v $cmd &> /dev/null; then
        MISSING_DEPS="$MISSING_DEPS $cmd"
    fi
done

if [ -n "$MISSING_DEPS" ]; then
    echo -e "${YELLOW}⚠️  Installing missing dependencies: $MISSING_DEPS${NC}"
    if command -v apt-get &> /dev/null; then
        apt-get update && apt-get install -y $MISSING_DEPS
    elif command -v yum &> /dev/null; then
        yum install -y $MISSING_DEPS
    fi
fi
echo -e "${GREEN}✅ Dependencies OK${NC}"

# Step 2: Create user and group
echo -e "${YELLOW}📝 Step 2: Creating user and group...${NC}"
if ! getent group $GROUP_NAME > /dev/null 2>&1; then
    groupadd -g $GROUP_ID $GROUP_NAME
    echo -e "${GREEN}✅ Group '$GROUP_NAME' created${NC}"
else
    echo -e "${GREEN}✅ Group '$GROUP_NAME' already exists${NC}"
fi

if ! id "$USER_NAME" > /dev/null 2>&1; then
    useradd -u $USER_ID -g $GROUP_ID -s /sbin/nologin -d "$INSTALL_DIR" "$USER_NAME"
    echo -e "${GREEN}✅ User '$USER_NAME' created${NC}"
else
    echo -e "${GREEN}✅ User '$USER_NAME' already exists${NC}"
fi

# Step 3: Create directory structure
echo -e "${YELLOW}📂 Step 3: Creating directory structure...${NC}"
mkdir -p "$INSTALL_DIR"/{bin,etc,data,docs,lib}
mkdir -p "$LOG_DIR"
mkdir -p "$CONFIG_DIR"

echo -e "${GREEN}✅ Directories created${NC}"

# Step 4: Install binary
echo -e "${YELLOW}📥 Step 4: Installing binary...${NC}"
cp "$BINARY_PATH" "$INSTALL_DIR/bin/rust-edge-compute"
chmod 755 "$INSTALL_DIR/bin/rust-edge-compute"
chown $USER_NAME:$GROUP_NAME "$INSTALL_DIR/bin/rust-edge-compute"
echo -e "${GREEN}✅ Binary installed${NC}"

# Step 5: Copy configuration
echo -e "${YELLOW}⚙️  Step 5: Configuring service...${NC}"

# Copy systemd service file
if [ -f "deploy/systemd/rust-edge-compute.service" ]; then
    cp deploy/systemd/rust-edge-compute.service "$SYSTEMD_DIR/"
    echo -e "${GREEN}✅ systemd service installed${NC}"
else
    echo -e "${YELLOW}⚠️  systemd service file not found${NC}"
fi

# Copy config if provided
if [ -n "$CONFIG_PATH" ] && [ -f "$CONFIG_PATH" ]; then
    cp "$CONFIG_PATH" "$CONFIG_DIR/production.toml"
    echo -e "${GREEN}✅ Configuration file installed${NC}"
else
    # Create default configuration
    cat > "$CONFIG_DIR/production.toml" <<'TOML'
[server]
host = "0.0.0.0"
port = 3000
workers = 4
keep_alive = 75

[logging]
level = "info"
format = "json"
output = "file"
path = "/var/log/edge-compute/app.log"

[security]
enable_auth = true
tls_enabled = false
TOML
    echo -e "${GREEN}✅ Default configuration created${NC}"
fi

# Create environment file
cat > "$CONFIG_DIR/edge-compute.env" <<'ENV'
# Application
RUST_LOG=info
RUST_BACKTRACE=1
APP_VERSION=0.1.0

# Server
LISTEN_HOST=0.0.0.0
LISTEN_PORT=3000
WORKERS=4

# Paths
CONFIG_PATH=/opt/edge-compute/etc/production.toml
LOG_PATH=/var/log/edge-compute/app.log
DATA_PATH=/opt/edge-compute/data
ENV

chown -R $USER_NAME:$GROUP_NAME "$INSTALL_DIR"
chown -R $USER_NAME:$GROUP_NAME "$LOG_DIR"
chown $USER_NAME:$GROUP_NAME "$CONFIG_DIR"
chmod 750 "$CONFIG_DIR"

echo -e "${GREEN}✅ Configuration files created${NC}"

# Step 6: Set file permissions
echo -e "${YELLOW}🔐 Step 6: Setting file permissions...${NC}"
chmod 750 "$INSTALL_DIR"
chmod 755 "$INSTALL_DIR/bin"
chmod 750 "$LOG_DIR"
chmod 644 "$SYSTEMD_DIR/rust-edge-compute.service"
echo -e "${GREEN}✅ Permissions set${NC}"

# Step 7: Enable systemd service
echo -e "${YELLOW}⚙️  Step 7: Registering systemd service...${NC}"
systemctl daemon-reload
systemctl enable rust-edge-compute
echo -e "${GREEN}✅ Service registered${NC}"

# Step 8: Create backup directory
echo -e "${YELLOW}💾 Step 8: Setting up backup directory...${NC}"
mkdir -p "$INSTALL_DIR/backups"
chown $USER_NAME:$GROUP_NAME "$INSTALL_DIR/backups"
chmod 750 "$INSTALL_DIR/backups"
echo -e "${GREEN}✅ Backup directory created${NC}"

# Summary
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Installation completed successfully!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}📋 Installation Summary:${NC}"
echo "  Installation Dir: $INSTALL_DIR"
echo "  Config Dir:       $CONFIG_DIR"
echo "  Log Dir:          $LOG_DIR"
echo "  Service User:     $USER_NAME:$GROUP_NAME"
echo ""
echo -e "${YELLOW}🚀 Next Steps:${NC}"
echo "  1. Review configuration:"
echo "     nano $CONFIG_DIR/production.toml"
echo ""
echo "  2. Start the service:"
echo "     sudo systemctl start rust-edge-compute"
echo ""
echo "  3. Check service status:"
echo "     sudo systemctl status rust-edge-compute"
echo ""
echo "  4. View logs:"
echo "     sudo journalctl -u rust-edge-compute -f"
echo ""
echo "  5. Health check:"
echo "     curl http://localhost:3000/api/v1/health"
echo ""
echo -e "${GREEN}Happy computing! 🎉${NC}"
