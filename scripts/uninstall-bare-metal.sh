#!/bin/bash

# 🗑️ Uninstall Script for Rust Edge Compute Framework
# Safe removal with backup preservation
# Usage: sudo ./scripts/uninstall-bare-metal.sh

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
INSTALL_DIR="/opt/edge-compute"
PRESERVE_DATA=true
PRESERVE_CONFIG=true
SERVICE_NAME="rust-edge-compute"

usage() {
    cat <<EOF
Usage: sudo $0 [OPTIONS]

Options:
    --purge              Remove all data and configurations
    --no-preserve        Don't preserve data and config
    --install-dir <DIR>  Installation directory (default: /opt/edge-compute)
    --force              Skip confirmation prompt
    --help               Show this help message

Examples:
    sudo ./scripts/uninstall-bare-metal.sh                # Safe uninstall
    sudo ./scripts/uninstall-bare-metal.sh --purge        # Complete removal
    sudo ./scripts/uninstall-bare-metal.sh --force        # No confirmation
EOF
    exit 0
}

FORCE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --purge|--no-preserve)
            PRESERVE_DATA=false
            PRESERVE_CONFIG=false
            shift
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
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

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🗑️  Rust Edge Compute Framework - Uninstall${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}📋 Uninstall Plan:${NC}"
echo "  Install Dir:      $INSTALL_DIR"
echo "  Service:          $SERVICE_NAME"
echo "  Preserve Data:    $PRESERVE_DATA"
echo "  Preserve Config:  $PRESERVE_CONFIG"
echo ""

# Confirmation
if [ "$FORCE" = false ]; then
    echo -e "${RED}⚠️  WARNING: This will uninstall the service!${NC}"
    read -p "Do you want to continue? (yes/no): " -r CONFIRM
    if [[ ! $CONFIRM =~ ^[Yy][Ee][Ss]$ ]]; then
        echo -e "${YELLOW}Uninstall cancelled.${NC}"
        exit 0
    fi
fi

# Step 1: Stop service
echo -e "${YELLOW}⛔ Step 1: Stopping service...${NC}"
if systemctl is-active --quiet $SERVICE_NAME; then
    systemctl stop $SERVICE_NAME
    sleep 2
    echo -e "${GREEN}✅ Service stopped${NC}"
else
    echo -e "${YELLOW}⚠️  Service not running${NC}"
fi

# Step 2: Disable service
echo -e "${YELLOW}🔧 Step 2: Disabling service...${NC}"
systemctl disable $SERVICE_NAME 2>/dev/null || true
echo -e "${GREEN}✅ Service disabled${NC}"

# Step 3: Remove systemd files
echo -e "${YELLOW}📋 Step 3: Removing systemd files...${NC}"
rm -f /etc/systemd/system/${SERVICE_NAME}.service
rm -f /etc/systemd/system/${SERVICE_NAME}.socket
systemctl daemon-reload
echo -e "${GREEN}✅ systemd files removed${NC}"

# Step 4: Create backup archive (if preserving)
if [ "$PRESERVE_DATA" = true ] || [ "$PRESERVE_CONFIG" = true ]; then
    echo -e "${YELLOW}💾 Step 4: Creating backup archive...${NC}"
    BACKUP_TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    BACKUP_FILE="/opt/edge-compute-backup_${BACKUP_TIMESTAMP}.tar.gz"
    
    mkdir -p /tmp/edge-compute-backup
    
    if [ "$PRESERVE_DATA" = true ] && [ -d "$INSTALL_DIR/data" ]; then
        cp -r "$INSTALL_DIR/data" /tmp/edge-compute-backup/ || true
    fi
    
    if [ "$PRESERVE_CONFIG" = true ] && [ -d "$INSTALL_DIR/etc" ]; then
        cp -r "$INSTALL_DIR/etc" /tmp/edge-compute-backup/ || true
    fi
    
    if [ "$PRESERVE_DATA" = true ] && [ -d "$INSTALL_DIR/backups" ]; then
        cp -r "$INSTALL_DIR/backups" /tmp/edge-compute-backup/ || true
    fi
    
    if [ "$(ls -A /tmp/edge-compute-backup)" ]; then
        tar -czf "$BACKUP_FILE" -C /tmp edge-compute-backup
        echo -e "${GREEN}✅ Backup created: $BACKUP_FILE${NC}"
    fi
    
    rm -rf /tmp/edge-compute-backup
fi

# Step 5: Remove application files
echo -e "${YELLOW}📁 Step 5: Removing application files...${NC}"

# Remove bin and lib directories
rm -rf "$INSTALL_DIR/bin"
rm -rf "$INSTALL_DIR/lib"
rm -rf "$INSTALL_DIR/docs"

if [ "$PRESERVE_DATA" = false ]; then
    rm -rf "$INSTALL_DIR/data"
    rm -rf "$INSTALL_DIR/backups"
fi

if [ "$PRESERVE_CONFIG" = false ]; then
    rm -rf "$INSTALL_DIR/etc"
fi

# Remove install directory if empty
if [ -z "$(ls -A "$INSTALL_DIR" 2>/dev/null)" ]; then
    rmdir "$INSTALL_DIR" 2>/dev/null || true
fi

echo -e "${GREEN}✅ Application files removed${NC}"

# Step 6: Remove log directory
echo -e "${YELLOW}📋 Step 6: Removing log files...${NC}"
rm -rf /var/log/edge-compute
echo -e "${GREEN}✅ Log files removed${NC}"

# Step 7: Remove user and group
echo -e "${YELLOW}👤 Step 7: Removing user and group...${NC}"
if id "edge-compute" > /dev/null 2>&1; then
    userdel edge-compute || true
fi

if getent group "edge-compute" > /dev/null 2>&1; then
    groupdel edge-compute || true
fi
echo -e "${GREEN}✅ User and group removed${NC}"

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Uninstall completed successfully!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

if [ "$PRESERVE_DATA" = true ] || [ "$PRESERVE_CONFIG" = true ]; then
    echo -e "${YELLOW}💾 Backup Information:${NC}"
    echo "  Backup file:  $BACKUP_FILE"
    echo "  Size:         $(du -h "$BACKUP_FILE" 2>/dev/null | awk '{print $1}')"
    echo ""
    echo -e "${YELLOW}To restore:${NC}"
    echo "  tar -xzf $BACKUP_FILE -C /"
    echo ""
fi

echo -e "${YELLOW}📋 Remaining:${NC}"
echo "  ✓ Source code: Still available in repository"
echo "  ✓ Configuration: Backed up (if preserved)"
echo "  ✓ Data: Backed up (if preserved)"
echo ""
echo -e "${GREEN}Thank you for using Rust Edge Compute Framework! 👋${NC}"
