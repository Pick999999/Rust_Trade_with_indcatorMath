@echo off
setlocal

:: ดึงวันที่และเดือนปัจจุบันด้วย WMIC (เนื่องจากเครื่องมีปัญหา PowerShell .NET)
for /f "tokens=2 delims==" %%I in ('wmic os get localdatetime /value') do set "DT=%%I"
set "MM=%DT:~4,2%"
set "DD=%DT:~6,2%"
set "DAY_MONTH=%DD%_%MM%"

:: รับค่า Version จากผู้ใช้งาน
set /p VERSION="Enter backup version (e.g. v1, 1.0.1): "

:: กำหนดชื่อ Folder เป้าหมาย (เช่น backup\v1_05_05)
set "BACKUP_DIR=backup\%VERSION%_%DAY_MONTH%"

echo =========================================
echo  Starting Backup to: %BACKUP_DIR%
echo =========================================

:: สร้างโฟลเดอร์ backup ถ้ายังไม่มี
if not exist "%BACKUP_DIR%" (
    mkdir "%BACKUP_DIR%"
)

:: สำรองข้อมูลโฟลเดอร์ src
echo.
echo [1/2] Backing up 'src' directory...
xcopy "src" "%BACKUP_DIR%\src\" /E /I /H /Y /C /Q

:: สำรองข้อมูลโฟลเดอร์ public
echo.
echo [2/2] Backing up 'public' directory...
xcopy "public" "%BACKUP_DIR%\public\" /E /I /H /Y /C /Q

echo.
echo =========================================
echo  Backup Completed Successfully!
echo  Location: %BACKUP_DIR%
echo =========================================
pause
