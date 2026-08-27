#!/bin/sh
set -e

# --- CONFIG ---
REPO="Rubencc96/crabber-tui"
BIN_NAME="crabber-tui"
DEST_DIR="~/.local/bin"
# ---------------------

echo "🦀 Installing $BIN_NAME..."

OS=$(uname -s | tr '[:upper:]' '[:lower:]')

if [ "$OS" = "linux" ]; then
    ASSET="crabber-linux-amd64"
elif [ "$OS" = "darwin" ]; then
    ASSET="crabber-macos-amd64"
else
    echo "❌ Detected OS is not supported by this installer: $OS"
    echo "Please, compile it from source code using cargo."
    exit 1
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ASSET"

echo "📥 Downloading latest version for $OS..."
curl -sL "$DOWNLOAD_URL" -o "/tmp/$BIN_NAME"
chmod +x "/tmp/$BIN_NAME"
mv "/tmp/$BIN_NAME" "$DEST_DIR/$BIN_NAME"

echo "✅ Installation complete!"
echo "🦀Write '$BIN_NAME' on the terminal to play🦀" 
