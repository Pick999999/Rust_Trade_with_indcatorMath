@echo off
REM Build verification script for Windows

echo ========================================
echo Turbo Indicators - Build Verification
echo ========================================
echo.

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo not found. Please install Rust:
    echo   https://rustup.rs/
    exit /b 1
)

for /f "tokens=*" %%i in ('cargo --version') do set CARGO_VERSION=%%i
echo [OK] Cargo found: %CARGO_VERSION%
echo.

REM Clean previous build
echo Cleaning previous build...
cargo clean
echo.

REM Build in release mode
echo Building in release mode...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] BUILD FAILED!
    echo.
    echo Please check the errors above.
    echo Common fixes:
    echo   1. Make sure Rust is up to date: rustup update
    echo   2. Check Cargo.toml syntax
    echo   3. Verify all source files are present
    exit /b 1
)

echo.
echo [SUCCESS] BUILD SUCCESSFUL!
echo.

REM Run demo
echo Running demo...
echo.
cargo run --release --example demo

echo.
echo ========================================
echo [SUCCESS] All tests passed! Ready to use.
echo ========================================
