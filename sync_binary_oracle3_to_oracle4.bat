@echo off
setlocal

echo =======================================================
echo    Sync Rust Binary: Oracle 3 to Oracle 4
echo =======================================================
echo.

set "KEY3=D:\Oracle\NewInstance-FreeTier_3-Boot\privatekey\ssh-key-2026-08-31.key"
set "KEY4=D:\Oracle\NewInstance-FreeTier-4\privateKey\ssh-key-2026-09-01.key"
set "IP3=161.118.203.228"
set "IP4=161.118.217.177"
set "USER=ubuntu"
set "SRC_PATH=/home/ubuntu/indicatorMultiplex/target/release/turbo-indicators"
set "DEST_DIR=/home/ubuntu/indicatorMultiplex/target/release"
set "DEST_TEMP=/home/ubuntu/indicatorMultiplex/target/release/turbo-indicators.new"
set "DEST_BIN=/home/ubuntu/indicatorMultiplex/target/release/turbo-indicators"

if not exist "%KEY3%" goto err_key3
if not exist "%KEY4%" goto err_key4

echo [1/3] Copying binary from Oracle 3 to Oracle 4...
echo       Source:      %IP3% (%SRC_PATH%)
echo       Destination: %IP4% (%DEST_TEMP%)
echo.

scp -3 -o StrictHostKeyChecking=no -i "%KEY3%" -i "%KEY4%" %USER%@%IP3%:%SRC_PATH% %USER%@%IP4%:%DEST_TEMP%
if errorlevel 1 goto err_scp

echo.
echo [2/3] Setting permissions and replacing binary on Oracle 4...
ssh -o StrictHostKeyChecking=no -i "%KEY4%" %USER%@%IP4% "chmod +x %DEST_TEMP% && mv -f %DEST_TEMP% %DEST_BIN% && ls -lh %DEST_BIN%"
if errorlevel 1 goto err_ssh

echo.
echo [3/3] Success! Binary installed on Oracle 4.
echo.
echo =======================================================
set /p RESTART="Do you want to restart service on Oracle 4 now? (Y/N): "

if /i "%RESTART%"=="Y" goto do_restart
goto end_success

:do_restart
echo.
echo Restarting tradeMultiplex.service...
ssh -o StrictHostKeyChecking=no -i "%KEY4%" %USER%@%IP4% "sudo systemctl restart tradeMultiplex.service && systemctl status tradeMultiplex.service --no-pager"
echo.
echo [SUCCESS] Service restarted successfully!
goto finish

:end_success
echo.
echo [INFO] Skipped restart. You can restart via web anytime.
goto finish

:err_key3
echo.
echo [ERROR] Private key for Oracle 3 not found at: %KEY3%
goto finish

:err_key4
echo.
echo [ERROR] Private key for Oracle 4 not found at: %KEY4%
goto finish

:err_scp
echo.
echo [ERROR] scp transfer failed. Please check if binary is built on Oracle 3.
goto finish

:err_ssh
echo.
echo [ERROR] Failed to set permissions or replace file on Oracle 4.
goto finish

:finish
echo =======================================================
pause
