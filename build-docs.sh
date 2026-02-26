#!/usr/bin/env bash
set -euo pipefail

echo "📚 Building tachyonfx documentation with live effect demos..."

# Create docs assets directory
DOCS_DIR="docs-assets"
mkdir -p "$DOCS_DIR"

# Check if WASM files already exist (e.g., committed to repo)
if [ -f "$DOCS_DIR/tachyonfx_renderer.js" ] && [ -f "$DOCS_DIR/tachyonfx_renderer_bg.wasm" ]; then
    echo "✅ Using existing tachyonfx-renderer assets from $DOCS_DIR/"
else
    # Download tachyonfx-renderer from npm
    echo "📦 Downloading tachyonfx-renderer from npm..."

    # Save current directory
    ORIGINAL_DIR=$(pwd)
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    cd "$TEMP_DIR"
    npm pack tachyonfx-renderer@0.4.0 >/dev/null 2>&1
    tar -xzf tachyonfx-renderer-*.tgz

    # Copy WASM and JS files to docs assets (using absolute path)
    cp package/tachyonfx_renderer.js "$ORIGINAL_DIR/$DOCS_DIR/"
    cp package/tachyonfx_renderer_bg.wasm "$ORIGINAL_DIR/$DOCS_DIR/"
    cp package/tachyonfx_renderer.d.ts "$ORIGINAL_DIR/$DOCS_DIR/" || true

    cd "$ORIGINAL_DIR"

    echo "✅ Downloaded tachyonfx-renderer assets to $DOCS_DIR/"
fi

# Build documentation with custom header
echo "🔨 Building documentation..."

RUSTDOCFLAGS="--html-in-header docs-assets/doc-header.html" \
cargo doc --no-deps

# Copy WASM files to target/doc/ so they're accessible
echo "📋 Copying WASM assets to target/doc/..."
mkdir -p target/doc/tachyonfx-renderer
cp "$DOCS_DIR"/* target/doc/tachyonfx-renderer/

echo "✅ Documentation built successfully!"
echo ""
echo "📖 To view the docs with live demos:"
echo ""
echo "   cd target/doc && python3 -m http.server 8000"
echo ""
echo "   Then open: http://localhost:8000/tachyonfx/index.html"
echo ""
echo "🌊 Live effect demos require HTTP server (CORS restriction prevents file:// access)"
