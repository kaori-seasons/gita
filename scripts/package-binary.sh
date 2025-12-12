#!/bin/bash

# 📦 Binary Packaging Script for Rust Edge Compute Framework
# Packages binary with dependencies and checksums
# Usage: ./scripts/package-binary.sh --version 0.1.0 --output release/

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Default values
VERSION=""
BINARY_PATH="target/x86_64-unknown-linux-gnu/release/rust-edge-compute"
OUTPUT_DIR="release"
TARGET="x86_64-unknown-linux-gnu"
INCLUDE_DEPS=true

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
    --version <VERSION>     Version number (required) e.g., 0.1.0
    --binary <PATH>         Binary path (default: target/x86_64-unknown-linux-gnu/release/rust-edge-compute)
    --target <TARGET>       Target platform (default: x86_64-unknown-linux-gnu)
    --output <DIR>          Output directory (default: release)
    --no-deps               Don't include dependencies list
    --help                  Show this help message

Examples:
    ./scripts/package-binary.sh --version 0.1.0 --output release/
    ./scripts/package-binary.sh --version 0.1.0 --target aarch64-unknown-linux-gnu
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --no-deps)
            INCLUDE_DEPS=false
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

# Validate inputs
if [ -z "$VERSION" ]; then
    echo -e "${RED}❌ Error: --version is required${NC}"
    exit 1
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Error: Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"
PACKAGE_NAME="rust-edge-compute-v${VERSION}-${TARGET}"
PACKAGE_DIR="$OUTPUT_DIR/$PACKAGE_NAME"
mkdir -p "$PACKAGE_DIR"

echo -e "${YELLOW}📦 Packaging Rust Edge Compute v${VERSION}${NC}"
echo "  Target:   $TARGET"
echo "  Binary:   $BINARY_PATH"
echo "  Output:   $PACKAGE_DIR"
echo ""

# Copy binary
echo -e "${YELLOW}📋 Copying binary...${NC}"
cp "$BINARY_PATH" "$PACKAGE_DIR/rust-edge-compute"
chmod +x "$PACKAGE_DIR/rust-edge-compute"

# Generate checksums
echo -e "${YELLOW}🔐 Generating checksums...${NC}"
BINARY_SIZE=$(du -h "$PACKAGE_DIR/rust-edge-compute" | cut -f1)

if command -v sha256sum &> /dev/null; then
    sha256sum "$PACKAGE_DIR/rust-edge-compute" > "$PACKAGE_DIR/SHA256SUMS"
fi

if command -v md5sum &> /dev/null; then
    md5sum "$PACKAGE_DIR/rust-edge-compute" > "$PACKAGE_DIR/MD5SUMS"
fi

# Create metadata file
echo -e "${YELLOW}📄 Creating metadata...${NC}"
cat > "$PACKAGE_DIR/METADATA.json" <<EOF
{
  "name": "rust-edge-compute",
  "version": "$VERSION",
  "target": "$TARGET",
  "binary": "rust-edge-compute",
  "size_bytes": $(stat -f%z "$PACKAGE_DIR/rust-edge-compute" 2>/dev/null || stat -c%s "$PACKAGE_DIR/rust-edge-compute"),
  "created_at": "$(date -u +'%Y-%m-%dT%H:%M:%SZ')",
  "checksum_sha256": "$(head -1 "$PACKAGE_DIR/SHA256SUMS" 2>/dev/null | awk '{print $1}')",
  "required_dependencies": [
    "glibc >= 2.17",
    "libssl >= 1.1.0",
    "curl (for health checks)"
  ]
}
EOF

# Create README
cat > "$PACKAGE_DIR/README.md" <<EOF
# Rust Edge Compute Framework v${VERSION}

## System Requirements

### Minimum
- OS: Ubuntu 20.04 LTS, CentOS 7+, or Debian 10+
- CPU: 2 cores
- RAM: 2GB
- Storage: 500MB

### Recommended
- OS: Ubuntu 20.04+ or 22.04 LTS
- CPU: 4+ cores with AVX2
- RAM: 8GB+
- Storage: 2TB SSD

## Installation

\`\`\`bash
sudo ./install-bare-metal.sh \\
  --binary ./rust-edge-compute \\
  --version $VERSION
\`\`\`

## Verification

\`\`\`bash
# Verify SHA256 checksum
sha256sum -c SHA256SUMS

# Test binary
./rust-edge-compute --version
\`\`\`

## Configuration

See DEPLOYMENT_GUIDE.md for detailed configuration instructions.

## Support

For issues and support, refer to the main project documentation.
EOF

# Copy additional files if they exist
if [ -d "deploy/config" ]; then
    echo -e "${YELLOW}📋 Copying configuration templates...${NC}"
    cp -r deploy/config "$PACKAGE_DIR/config" || true
fi

if [ -d "deploy/systemd" ]; then
    echo -e "${YELLOW}⚙️  Copying systemd files...${NC}"
    cp -r deploy/systemd "$PACKAGE_DIR/systemd" || true
fi

# Create distribution package
echo -e "${YELLOW}📦 Creating distribution package...${NC}"
cd "$OUTPUT_DIR"
tar -czf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME"
tar -cjf "${PACKAGE_NAME}.tar.bz2" "$PACKAGE_NAME"

# Generate package checksums
if command -v sha256sum &> /dev/null; then
    sha256sum "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"
    sha256sum "${PACKAGE_NAME}.tar.bz2" > "${PACKAGE_NAME}.tar.bz2.sha256"
fi

cd - > /dev/null

echo ""
echo -e "${GREEN}✅ Packaging completed!${NC}"
echo ""
echo -e "${GREEN}📦 Package Information:${NC}"
echo "  Package:     $PACKAGE_NAME"
echo "  Location:    $OUTPUT_DIR"
echo "  Binary Size: $BINARY_SIZE"
echo "  Archives:"
ls -lh "$OUTPUT_DIR/${PACKAGE_NAME}".tar.* | awk '{print "    " $9 " (" $5 ")"}'
echo ""
echo -e "${GREEN}✅ Ready for deployment!${NC}"
