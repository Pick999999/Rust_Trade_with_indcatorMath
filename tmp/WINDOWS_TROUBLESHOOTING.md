# Windows Build Troubleshooting Guide

## Error: "The process cannot access the file" (os error 32)

This error occurs when a file is locked by another process.

### Quick Fixes (Try in order):

### 1. Clean and Rebuild
```batch
cargo clean
cargo build --release
```

### 2. Close All Programs
- Close VS Code / Visual Studio
- Close any running Rust programs
- Close Windows Explorer if browsing target folder
- Close antivirus real-time scanning temporarily

### 3. Kill Rust Processes
```batch
taskkill /F /IM rust-analyzer.exe
taskkill /F /IM rls.exe
taskkill /F /IM cargo.exe
```

### 4. Delete target folder manually
```batch
rmdir /s /q target
cargo build --release
```

### 5. Build without parallel compilation
```batch
cargo build --release -j 1
```

### 6. Disable Windows Defender for the folder
```
Windows Security → Virus & threat protection → 
Manage settings → Add or remove exclusions → 
Add folder: D:\Rust\turbo-indicators
```

### 7. Use different drive (if possible)
Sometimes Windows has issues with specific drives.

## Error: Missing Visual Studio Build Tools

If you see errors about "link.exe" or MSVC:

1. Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/
2. Install "Desktop development with C++"
3. Restart terminal
4. Try build again

## Slow Build on Windows

### Enable incremental compilation:
```batch
set CARGO_INCREMENTAL=1
cargo build --release
```

### Use SSD if possible
Move project to SSD drive for faster builds.

## Build Succeeds but Demo Won't Run

### Error: "DLL not found"
Install Visual C++ Redistributable:
https://aka.ms/vs/17/release/vc_redist.x64.exe

## Alternative: WSL2 (Windows Subsystem for Linux)

If Windows keeps having issues, use WSL2:

```bash
# In PowerShell (Admin)
wsl --install

# Restart computer, then in WSL:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Extract and build
tar -xzf turbo-indicators.tar.gz
cd indicators
cargo build --release
cargo run --release --example demo
```

## Still Having Issues?

Try the minimal build:

```batch
# Just build the library (no examples)
cargo build --release --lib

# Then run demo separately
cargo run --release --example demo
```

---

## Working Configuration

Tested successfully on:
- Windows 10/11 with Visual Studio 2022 Build Tools
- Rust 1.75+
- No antivirus interference
- SSD drive
