#!/bin/bash
set -e

echo "🔨 Building Bridgelet Core contracts..."

cargo build --target wasm32-unknown-unknown --release

echo "✅ Build complete!"
echo ""
echo "📦 Built WASMs:"
find contracts -name "*.wasm" -path "*/wasm32-unknown-unknown/release/*" | while read f; do
    echo "  $(du -sh "$f" | cut -f1)  $f"
done
