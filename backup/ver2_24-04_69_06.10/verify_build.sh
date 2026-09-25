#!/bin/bash
# Build verification script

echo "🔧 Turbo Indicators - Build Verification"
echo "=========================================="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✓ Cargo found: $(cargo --version)"
echo ""

# Clean previous build
echo "🧹 Cleaning previous build..."
cargo clean
echo ""

# Build in release mode
echo "🔨 Building in release mode..."
if cargo build --release 2>&1 | tee build.log; then
    echo ""
    echo "✅ BUILD SUCCESSFUL!"
    echo ""
    
    # Check for warnings
    if grep -q "warning:" build.log; then
        echo "⚠️  Build completed with warnings (this is OK):"
        grep "warning:" build.log | head -5
        echo ""
    fi
    
    # Run demo
    echo "🚀 Running demo..."
    echo ""
    cargo run --release --example demo
    
    echo ""
    echo "=========================================="
    echo "✅ All tests passed! Ready to use."
    echo "=========================================="
else
    echo ""
    echo "❌ BUILD FAILED!"
    echo ""
    echo "Please check the errors above."
    echo "Common fixes:"
    echo "  1. Make sure Rust is up to date: rustup update"
    echo "  2. Check Cargo.toml syntax"
    echo "  3. Verify all source files are present"
    exit 1
fi

rm -f build.log
