@echo off
rem Builds the release bundle: installer.exe + walkupie.exe in a zip.
cd /d "%~dp0"
cargo build --release || exit /b 1
if not exist dist mkdir dist
if exist "dist\WalkUpie-Installer.zip" del "dist\WalkUpie-Installer.zip"
powershell -NoProfile -Command "Compress-Archive -Force -Path target\release\installer.exe,target\release\walkupie.exe -DestinationPath dist\WalkUpie-Installer.zip"
echo.
echo Bundle: dist\WalkUpie-Installer.zip
echo Share both files, user runs installer.exe and answers the 4 prompts.