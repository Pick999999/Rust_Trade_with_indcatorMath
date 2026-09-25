@echo off
setlocal

echo =======================================================
echo    Download Rust Binary: Oracle 3 to PC (tmp\build)
echo =======================================================
echo.

set "KEY3=D:\Oracle\NewInstance-FreeTier_3-Boot\privatekey\ssh-key-2026-08-31.key"
set "IP3=161.118.203.228"
set "USER=ubuntu"
set "SRC_PATH=/home/ubuntu/indicatorMultiplex/target/release/turbo-indicators"
set "DEST_DIR=%~dp0tmp\build"
set "DEST_FILE=%DEST_DIR%\turbo-indicators"

if not exist "%KEY3%" goto err_key3

if not exist "%DEST_DIR%" (
    mkdir "%DEST_DIR%"
)

echo Downloading binary from Oracle 3...
echo   Source:      %IP3% (%SRC_PATH%)
echo   Destination: %DEST_FILE%
echo.

scp -o StrictHostKeyChecking=no -i "%KEY3%" %USER%@%IP3%:%SRC_PATH% "%DEST_FILE%"
if errorlevel 1 goto err_scp

echo.
echo =======================================================
echo [SUCCESS] Download completed!
echo File saved to: %DEST_FILE%
echo =======================================================
goto finish

:err_key3
echo [ERROR] Private key for Oracle 3 not found at: %KEY3%
goto finish

:err_scp
echo.
echo [ERROR] scp download failed. Please check if build is complete on Oracle 3.
goto finish

:finish
echo.
pause
