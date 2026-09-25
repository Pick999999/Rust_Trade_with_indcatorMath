# 🚀 Quick Start Guide

## Prerequisites

1. **Install Rust** (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. **Verify installation**:
```bash
rustc --version
cargo --version
```

## Installation Steps

### Method 1: Extract and Build

```bash
# Extract the archive
tar -xzf turbo-indicators.tar.gz
cd indicators

# Build in release mode (optimized)
cargo build --release

# Run the demo
cargo run --release --example demo
```

### Method 2: Enable SIMD (Recommended for maximum performance)

```bash
cd indicators

# Build with CPU-specific optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run demo
RUSTFLAGS="-C target-cpu=native" cargo run --release --example demo
```

## Using in Your Project

### Option A: Local dependency

In your project's `Cargo.toml`:

```toml
[dependencies]
turbo-indicators = { path = "../turbo-indicators/indicators" }
```

### Option B: Copy the source

Copy the entire `indicators/src` directory into your project.

## Quick Test

Create a new file `test.rs`:

```rust
use turbo_indicators::*;

fn main() {
    // Simple test data
    let prices = vec![
        100.0, 101.0, 102.5, 101.8, 103.0,
        104.2, 103.5, 105.0, 106.5, 107.0,
        108.2, 107.5, 109.0, 110.5, 111.0,
        110.2, 111.5, 112.0, 111.8, 113.0,
    ];

    // Calculate RSI
    let rsi = RSI::new(14);
    let rsi_values = rsi.compute(&prices);
    
    println!("RSI values: {:?}", rsi_values);
    
    if let Some(&last_rsi) = rsi_values.last() {
        println!("\nLast RSI: {:.2}", last_rsi);
        
        if last_rsi > 70.0 {
            println!("Status: OVERBOUGHT 🔴");
        } else if last_rsi < 30.0 {
            println!("Status: OVERSOLD 🟢");
        } else {
            println!("Status: NEUTRAL ⚪");
        }
    }
}
```

Run it:
```bash
cargo run --release
```

## Project Structure

```
indicators/
├── Cargo.toml              # Project configuration
├── README.md               # Main documentation
├── USAGE.md               # Detailed usage examples
├── PERFORMANCE.md         # Performance benchmarks
├── src/
│   ├── lib.rs            # Main library entry
│   ├── indicators.rs     # Core indicator implementations
│   ├── simd.rs          # SIMD-optimized math
│   └── streaming.rs     # Real-time streaming engine
└── examples/
    └── demo.rs          # Comprehensive demo
```

## Common Commands

```bash
# Build
cargo build --release

# Run tests (when you add them)
cargo test

# Run demo
cargo run --release --example demo

# Check for errors without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Troubleshooting

### Issue: "cargo: command not found"
**Solution**: Install Rust or add cargo to PATH:
```bash
source $HOME/.cargo/env
```

### Issue: Slow performance
**Solution**: Make sure you're building in release mode:
```bash
cargo build --release  # NOT cargo build
```

### Issue: Want maximum performance
**Solution**: Enable CPU-specific optimizations:
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Next Steps

1. Read `USAGE.md` for detailed examples
2. Check `PERFORMANCE.md` for optimization tips
3. Modify `examples/demo.rs` to test with your data
4. Start integrating into your trading system!

## Support

For questions or issues:
- Check the documentation in the `indicators/` directory
- Review the example code in `examples/demo.rs`
- Examine the implementation in `src/` files

Happy trading! 📈🚀
