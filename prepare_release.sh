#!/bin/bash

echo "Preparing BitcoinZ Light CLI for release..."

# Get version from Cargo.toml
VERSION=$(grep "^version" cli/Cargo.toml | head -1 | cut -d'"' -f2)
echo "Version: $VERSION"

# Create release directory
RELEASE_DIR="releases/v${VERSION}"
mkdir -p "$RELEASE_DIR"

# Get system info
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Create platform-specific archive
if [ "$OS" = "darwin" ]; then
    PLATFORM="macos-${ARCH}"
elif [ "$OS" = "linux" ]; then
    PLATFORM="linux-${ARCH}"
else
    PLATFORM="${OS}-${ARCH}"
fi

ARCHIVE_NAME="bitcoinz-light-cli-v${VERSION}-${PLATFORM}"

# Check if binary exists
if [ ! -f "target/release/bitcoinz-light-cli" ]; then
    echo "Release binary not found. Building..."
    cargo build --release
fi

# Copy binary and create archive
echo "Creating release archive: ${ARCHIVE_NAME}.tar.gz"
mkdir -p "$RELEASE_DIR/$ARCHIVE_NAME"
cp target/release/bitcoinz-light-cli "$RELEASE_DIR/$ARCHIVE_NAME/"
cp README.md "$RELEASE_DIR/$ARCHIVE_NAME/"
cp LICENSE "$RELEASE_DIR/$ARCHIVE_NAME/"
cp USER_GUIDE.md "$RELEASE_DIR/$ARCHIVE_NAME/"

cd "$RELEASE_DIR"
tar -czf "${ARCHIVE_NAME}.tar.gz" "$ARCHIVE_NAME"
rm -rf "$ARCHIVE_NAME"

echo "Release archive created: $RELEASE_DIR/${ARCHIVE_NAME}.tar.gz"

# Calculate checksum
shasum -a 256 "${ARCHIVE_NAME}.tar.gz" > "${ARCHIVE_NAME}.tar.gz.sha256"

echo "Checksum created: $RELEASE_DIR/${ARCHIVE_NAME}.tar.gz.sha256"

cd ../..

echo ""
echo "Release preparation complete!"
echo "Archive: releases/v${VERSION}/${ARCHIVE_NAME}.tar.gz"