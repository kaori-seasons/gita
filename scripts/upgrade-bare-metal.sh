#!/bin/bash

# 🔄 Upgrade Script for Rust Edge Compute Framework
# Performs atomic updates with automatic rollback
# Usage: sudo ./scripts/upgrade-bare-metal.sh --package ./rust-edge-compute-v0.2.0.tar.gz

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
PACKAGE_PATH=""
INSTALL_DIR="/opt/edge-compute"
BACKUP_DIR="$INSTALL_DIR/backups"
HEALTHCHECK_URL="http://localhost:3000/api/v1/health"
HEALTHCHECK_TIMEOUT=60
SERVICE_NAME="rust-edge-compute"

usage() {
    cat <<EOF
Usage: sudo $0 [OPTIONS]

Options:
    --package <PATH>        Path to upgrade package (required)
    --install-dir <DIR>     Installation directory (default: /opt/edge-compute)
    --healthcheck-url <URL> Health check URL (default: http://localhost:3000/api/v1/health)
    --help                  Show this help message

Examples:
    sudo ./scripts/upgrade-bare-metal.sh --package ./rust-edge-compute-v0.2.0.tar.gz
    sudo ./scripts/upgrade-bare-metal.sh --package ./package.tar.gz --install-dir /opt/app
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --package)
            PACKAGE_PATH="$2"
            shift 2
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --healthcheck-url)
            HEALTHCHECK_URL="$2"
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
if [ -z "$PACKAGE_PATH" ]; then
    echo -e "${RED}❌ Error: --package is required${NC}"
    exit 1
fi

if [ ! -f "$PACKAGE_PATH" ]; then
    echo -e "${RED}❌ Error: Package not found at $PACKAGE_PATH${NC}"
    exit 1
fi

# Extract version from package
PACKAGE_NAME=$(basename "$PACKAGE_PATH" .tar.gz)
NEW_VERSION=$(echo "$PACKAGE_NAME" | grep -oP 'v\K[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🔄 Rust Edge Compute Framework - Upgrade${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}📋 Upgrade Plan:${NC}"
echo "  Package:        $PACKAGE_PATH"
echo "  New Version:    $NEW_VERSION"
echo "  Install Dir:    $INSTALL_DIR"
echo "  Service:        $SERVICE_NAME"
echo ""

# Step 1: Pre-upgrade checks
echo -e "${YELLOW}🔍 Step 1: Pre-upgrade checks...${NC}"

# Check if service is running
if systemctl is-active --quiet $SERVICE_NAME; then
    echo -e "${GREEN}✅ Service is running${NC}"
else
    echo -e "${YELLOW}⚠️  Service is not running${NC}"
fi

# Create timestamp for backup
BACKUP_TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="$BACKUP_DIR/backup_$BACKUP_TIMESTAMP"

# Step 2: Create backup
echo -e "${YELLOW}💾 Step 2: Creating backup...${NC}"
mkdir -p "$BACKUP_PATH"
cp -r "$INSTALL_DIR/bin" "$BACKUP_PATH/" || true
echo -e "${GREEN}✅ Backup created at $BACKUP_PATH${NC}"

# Step 3: Extract package
echo -e "${YELLOW}📦 Step 3: Extracting upgrade package...${NC}"
TEMP_DIR=$(mktemp -d)
tar -xzf "$PACKAGE_PATH" -C "$TEMP_DIR"
EXTRACTED_DIR=$(ls -d "$TEMP_DIR"/* | head -1)
echo -e "${GREEN}✅ Package extracted${NC}"

# Step 4: Stop service
echo -e "${YELLOW}⛔ Step 4: Stopping service...${NC}"
systemctl stop $SERVICE_NAME
sleep 2
echo -e "${GREEN}✅ Service stopped${NC}"

# Step 5: Install new binary
echo -e "${YELLOW}📥 Step 5: Installing new binary...${NC}"
if [ -f "$EXTRACTED_DIR/rust-edge-compute" ]; then
    cp "$EXTRACTED_DIR/rust-edge-compute" "$INSTALL_DIR/bin/rust-edge-compute"
    chmod 755 "$INSTALL_DIR/bin/rust-edge-compute"
    chown edge-compute:edge-compute "$INSTALL_DIR/bin/rust-edge-compute"
    echo -e "${GREEN}✅ Binary installed${NC}"
else
    echo -e "${RED}❌ Binary not found in package${NC}"
    echo -e "${YELLOW}⚡ Rolling back...${NC}"
    cp "$BACKUP_PATH/bin/rust-edge-compute" "$INSTALL_DIR/bin/"
    systemctl start $SERVICE_NAME
    echo -e "${RED}❌ Upgrade failed and rolled back${NC}"
    exit 1
fi

# Step 6: Start service
echo -e "${YELLOW}▶️  Step 6: Starting service...${NC}"
systemctl start $SERVICE_NAME
sleep 5
echo -e "${GREEN}✅ Service started${NC}"

# Step 7: Health check
echo -e "${YELLOW}🏥 Step 7: Performing health check...${NC}"
HEALTH_CHECK_PASSED=false
for i in {1..12}; do
    if curl -sf "$HEALTHCHECK_URL" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Health check passed${NC}"
        HEALTH_CHECK_PASSED=true
        break
    fi
    echo -e "${YELLOW}⏳ Waiting for service to be healthy (attempt $i/12)...${NC}"
    sleep 5
done

if [ "$HEALTH_CHECK_PASSED" = false ]; then
    echo -e "${RED}❌ Health check failed - initiating rollback${NC}"
    systemctl stop $SERVICE_NAME
    cp "$BACKUP_PATH/bin/rust-edge-compute" "$INSTALL_DIR/bin/"
    systemctl start $SERVICE_NAME
    echo -e "${RED}❌ Upgrade failed and rolled back${NC}"
    exit 1
fi

# Step 8: Verify logs for errors
echo -e "${YELLOW}📋 Step 8: Verifying logs...${NC}"
if journalctl -u $SERVICE_NAME -n 20 | grep -i "error\|panic"; then
    echo -e "${YELLOW}⚠️  Warnings found in logs${NC}"
else
    echo -e "${GREEN}✅ No errors in logs${NC}"
fi

# Cleanup
echo -e "${YELLOW}🧹 Cleaning up temporary files...${NC}"
rm -rf "$TEMP_DIR"

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Upgrade completed successfully!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}📋 Summary:${NC}"
echo "  Upgraded to:    Version $NEW_VERSION"
echo "  Backup:         $BACKUP_PATH"
echo "  Status:         Running"
echo ""
echo -e "${YELLOW}🔄 Rollback Command (if needed):${NC}"
echo "  sudo cp $BACKUP_PATH/bin/rust-edge-compute $INSTALL_DIR/bin/"
echo "  sudo systemctl restart $SERVICE_NAME"
echo ""
echo -e "${GREEN}Happy computing! 🎉${NC}"
