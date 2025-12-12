#!/bin/bash

# 📦 Version Manager for Rust Edge Compute Framework
# Tracks versions, upgrade history, and compatibility
# Usage: ./scripts/version-manager.sh [COMMAND] [OPTIONS]

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
INSTALL_DIR="/opt/edge-compute"
VERSION_FILE="$INSTALL_DIR/VERSION"
HISTORY_FILE="$INSTALL_DIR/version_history.log"
BACKUP_DIR="$INSTALL_DIR/backups"

usage() {
    cat <<EOF
Usage: $0 [COMMAND] [OPTIONS]

Commands:
    current              Show current version
    history              Show version history
    list-backups         List available backup versions
    check-updates        Check for available updates
    compatibility        Check version compatibility
    info                 Show detailed version info
    help                 Show this help message

Examples:
    ./scripts/version-manager.sh current
    ./scripts/version-manager.sh history
    ./scripts/version-manager.sh list-backups
    ./scripts/version-manager.sh compatibility --from 0.1.0 --to 0.2.0
EOF
    exit 0
}

# Get current version
get_current_version() {
    if [ -f "$VERSION_FILE" ]; then
        cat "$VERSION_FILE"
    else
        echo "unknown"
    fi
}

# Show current version
show_current() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}📦 Current Version Information${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    CURRENT=$(get_current_version)
    echo "  Version:      $CURRENT"
    
    # Get binary info
    if [ -f "$INSTALL_DIR/bin/rust-edge-compute" ]; then
        BINARY_SIZE=$(du -h "$INSTALL_DIR/bin/rust-edge-compute" | awk '{print $1}')
        BINARY_DATE=$(date -r "$INSTALL_DIR/bin/rust-edge-compute" '+%Y-%m-%d %H:%M:%S')
        echo "  Binary Size:  $BINARY_SIZE"
        echo "  Installed:    $BINARY_DATE"
        
        # Try to get version from binary
        if "$INSTALL_DIR/bin/rust-edge-compute" --version > /dev/null 2>&1; then
            BIN_VERSION=$("$INSTALL_DIR/bin/rust-edge-compute" --version 2>/dev/null || echo "unknown")
            echo "  Binary Ver:   $BIN_VERSION"
        fi
    fi
    
    # Service info
    if systemctl is-active --quiet rust-edge-compute 2>/dev/null; then
        echo "  Service:      Active ✅"
        UPTIME=$(systemctl show rust-edge-compute --property=ActiveEnterTimestamp --value)
        echo "  Active Since: $UPTIME"
    else
        echo "  Service:      Inactive ⛔"
    fi
}

# Show version history
show_history() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}📜 Version History${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    if [ -f "$HISTORY_FILE" ]; then
        cat "$HISTORY_FILE"
    else
        echo -e "${YELLOW}No version history found${NC}"
    fi
}

# List backup versions
list_backups() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}💾 Available Backup Versions${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    if [ ! -d "$BACKUP_DIR" ]; then
        echo -e "${YELLOW}No backups found${NC}"
        return
    fi
    
    echo ""
    ls -1d "$BACKUP_DIR"/backup_* 2>/dev/null | while read -r backup; do
        BACKUP_NAME=$(basename "$backup")
        BACKUP_SIZE=$(du -sh "$backup" 2>/dev/null | awk '{print $1}')
        BACKUP_DATE=$(stat -c %y "$backup" 2>/dev/null | cut -d' ' -f1-2)
        
        echo "  ✓ $BACKUP_NAME"
        echo "    Size: $BACKUP_SIZE, Date: $BACKUP_DATE"
    done
    
    if [ -z "$(ls -1d "$BACKUP_DIR"/backup_* 2>/dev/null)" ]; then
        echo -e "${YELLOW}No backups found${NC}"
    fi
}

# Check for updates
check_updates() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}🔄 Checking for Updates${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    CURRENT=$(get_current_version)
    echo "  Current Version: $CURRENT"
    echo ""
    
    # Try to get latest version from remote (placeholder)
    UPDATE_SERVER="https://updates.example.com/api/latest"
    
    if command -v curl &> /dev/null; then
        echo -e "${YELLOW}Contacting update server: $UPDATE_SERVER${NC}"
        
        # Note: This would require a real update server
        echo -e "${YELLOW}⚠️  Update server not configured${NC}"
        echo ""
        echo -e "${BLUE}To enable updates:${NC}"
        echo "  1. Configure UPDATE_SERVER in /opt/edge-compute/etc/edge-compute.env"
        echo "  2. Set up auto-update check with cron or systemd timer"
        echo ""
    else
        echo -e "${YELLOW}⚠️  curl not installed${NC}"
    fi
}

# Check version compatibility
check_compatibility() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}🔍 Version Compatibility Check${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # Placeholder compatibility matrix
    cat <<'EOF'
Version Compatibility Matrix:

0.1.x Compatible with:
  ✓ 0.1.0 - Initial release
  ✓ Supports direct upgrade to 0.2.x
  ✓ Downgrade to 0.1.x not supported

0.2.x Compatible with:
  ✓ 0.2.0 and higher
  ✓ Compatible upgrades from 0.1.x
  ✓ Can downgrade to 0.1.x (with data backup)

Known Issues:
  • Database format changed in 0.2.0
  • Configuration file migration needed
  • API versioning: v1 supported in all versions

Breaking Changes:
  0.2.0 → Database schema update required
  0.3.0 → Configuration format change
EOF
}

# Show detailed info
show_detailed_info() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}📊 Detailed Version Information${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    show_current
    echo ""
    echo -e "${YELLOW}Installed Components:${NC}"
    
    # Check components
    COMPONENTS=0
    if [ -f "$INSTALL_DIR/bin/rust-edge-compute" ]; then
        echo "  ✓ Main binary"
        ((COMPONENTS++))
    fi
    
    if [ -f "$INSTALL_DIR/etc/production.toml" ]; then
        echo "  ✓ Production configuration"
        ((COMPONENTS++))
    fi
    
    if [ -d "$INSTALL_DIR/data" ]; then
        echo "  ✓ Data directory"
        ((COMPONENTS++))
    fi
    
    if [ -d "$INSTALL_DIR/backups" ]; then
        echo "  ✓ Backup directory"
        ((COMPONENTS++))
    fi
    
    echo ""
    echo "  Total components: $COMPONENTS/4"
    
    echo ""
    echo -e "${YELLOW}System Resources:${NC}"
    if command -v systemctl &> /dev/null; then
        MEMORY=$(systemctl show rust-edge-compute --property=MemoryCurrent --value 2>/dev/null || echo "unknown")
        if [ "$MEMORY" != "unknown" ]; then
            MEMORY_MB=$((MEMORY / 1024 / 1024))
            echo "  Memory: ${MEMORY_MB}MB"
        fi
    fi
}

# Main command handling
if [ $# -eq 0 ]; then
    usage
fi

COMMAND="$1"
shift || true

case "$COMMAND" in
    current)
        show_current
        ;;
    history)
        show_history
        ;;
    list-backups)
        list_backups
        ;;
    check-updates)
        check_updates
        ;;
    compatibility)
        check_compatibility
        ;;
    info)
        show_detailed_info
        ;;
    help)
        usage
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        usage
        ;;
esac
