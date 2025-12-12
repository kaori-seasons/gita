#!/bin/bash

# 🖥️ Release Build Script for Rust Edge Compute Framework
# Supports optimized builds for different architectures
# Usage: ./scripts/build-release.sh [OPTIONS]

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
TARGET="x86_64-unknown-linux-gnu"
OPTIMIZE=false
LTO=true
STRIP=true
OUTPUT_DIR="target/release"
PROFILE="release"

# Help message
usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
    --target <TARGET>       Build target (default: x86_64-unknown-linux-gnu)
                           Options: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, armv7-unknown-linux-gnueabihf
    --optimize             Enable aggressive optimizations
    --no-lto               Disable LTO (Link Time Optimization)
    --no-strip             Don't strip symbols from binary
    --output <DIR>         Output directory (default: target/release)
    --profile <PROFILE>    Build profile (default: release)
    --help                 Show this help message

Examples:
    ./scripts/build-release.sh                              # Standard release build
    ./scripts/build-release.sh --target x86_64-unknown-linux-gnu --optimize
    ./scripts/build-release.sh --target aarch64-unknown-linux-gnu
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --optimize)
            OPTIMIZE=true
            shift
            ;;
        --no-lto)
            LTO=false
            shift
            ;;
        --no-strip)
            STRIP=false
            shift
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --profile)
            PROFILE="$2"
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

# Verify cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: cargo not found. Please install Rust toolchain.${NC}"
    exit 1
fi

# Display build configuration
echo -e "${YELLOW}📦 Build Configuration${NC}"
echo "  Target:     $TARGET"
echo "  Optimize:   $OPTIMIZE"
echo "  LTO:        $LTO"
echo "  Strip:      $STRIP"
echo "  Output:     $OUTPUT_DIR"
echo "  Profile:    $PROFILE"
echo ""

# Setup Cargo.toml overrides for optimization
if [ "$OPTIMIZE" = true ] && [ "$LTO" = true ]; then
    echo -e "${YELLOW}⚙️  Setting up optimization flags...${NC}"
    
    # Create or update .cargo/config.toml for optimal compilation
    mkdir -p .cargo
    cat > .cargo/config.toml <<'CARGO_CONFIG'
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld", "-C", "target-cpu=native"]

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"
CARGO_CONFIG
fi

# Install target if specified and not native
if [ "$TARGET" != "x86_64-unknown-linux-gnu" ]; then
    echo -e "${YELLOW}🔧 Installing target: $TARGET${NC}"
    rustup target add "$TARGET" 2>/dev/null || true
fi

# Build the project
echo -e "${YELLOW}🔨 Building Rust Edge Compute Framework...${NC}"
echo ""

CARGO_FLAGS=""
if [ "$PROFILE" != "release" ]; then
    CARGO_FLAGS="--profile $PROFILE"
fi

cargo build \
    --release \
    --target "$TARGET" \
    $CARGO_FLAGS \
    --all

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Build completed successfully!${NC}"
    
    # Find and display binary locations
    BINARY_PATH="target/$TARGET/release/rust-edge-compute"
    if [ -f "$BINARY_PATH" ]; then
        BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
        echo ""
        echo -e "${GREEN}📦 Binary Information:${NC}"
        echo "  Path:  $BINARY_PATH"
        echo "  Size:  $BINARY_SIZE"
        
        # Optional: Strip symbols if requested
        if [ "$STRIP" = true ] && [ -f "$BINARY_PATH" ]; then
            echo -e "${YELLOW}📉 Stripping symbols...${NC}"
            strip "$BINARY_PATH" 2>/dev/null || true
            STRIPPED_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
            echo "  Stripped Size: $STRIPPED_SIZE"
        fi
        
        echo ""
        echo -e "${GREEN}🎉 Build successful! Ready for packaging.${NC}"
        exit 0
    else
        echo -e "${RED}❌ Error: Binary not found at $BINARY_PATH${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Build failed!${NC}"
    exit 1
fi
