@echo off
REM ============================================================================
REM Debug TCP Connection - Spresense Security Camera Viewer
REM ============================================================================
REM
REM This script runs the GUI application with detailed logging enabled.
REM Use this when troubleshooting TCP connection issues.
REM
REM ============================================================================

echo ============================================================================
echo  Spresense Security Camera Viewer - TCP Debug Mode
echo ============================================================================
echo.
echo Starting GUI with debug logging enabled...
echo.
echo IMPORTANT:
echo   1. Make sure Spresense is running security_camera application
echo   2. Check Spresense serial console for IP address
echo   3. Example: "WiFi connected! IP: 192.168.137.247"
echo   4. Enter that IP address in the GUI
echo.
echo This window will show detailed logs.
echo Press Ctrl+C to stop.
echo.
echo ============================================================================
echo.

REM Set Rust log level to debug
set RUST_LOG=debug

REM Run GUI application
security_camera_gui.exe

echo.
echo ============================================================================
echo Application stopped
echo ============================================================================
pause
