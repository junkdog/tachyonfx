#!/usr/bin/env bash
set -euo pipefail

if [ ! -d "target/doc/tachyonfx" ]; then
    echo "❌ Documentation not found. Run ./build-docs.sh first."
    exit 1
fi

echo "🌐 Serving documentation at http://localhost:8000"
echo "📖 Open: http://localhost:8000/tachyonfx/index.html"
echo "🌊 Live effect demos:"
echo "   • http://localhost:8000/tachyonfx/fx/fn.slide_in.html"
echo "   • http://localhost:8000/tachyonfx/fx/fn.sweep_in.html"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

cd target/doc
python3 -m http.server 8000
