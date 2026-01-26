@echo off
REM Spresense Security Camera Viewer - TCP Connection Script
REM Version: Phase 7 (WiFi/TCP Support)

echo ================================================================================
echo   Spresense Security Camera Viewer - TCP Mode
echo ================================================================================
echo.

REM Default settings
set DEFAULT_HOST=192.168.137.50
set DEFAULT_PORT=8888

REM Prompt for Spresense IP address
echo Enter Spresense IP address (default: %DEFAULT_HOST%):
set /p HOST_INPUT=
if "%HOST_INPUT%"=="" set HOST_INPUT=%DEFAULT_HOST%

echo Enter TCP port (default: %DEFAULT_PORT%):
set /p PORT_INPUT=
if "%PORT_INPUT%"=="" set PORT_INPUT=%DEFAULT_PORT%

echo.
echo ================================================================================
echo Connecting to Spresense...
echo   Host: %HOST_INPUT%
echo   Port: %PORT_INPUT%
echo ================================================================================
echo.

REM Launch viewer
security_camera_viewer.exe --transport tcp --tcp-host %HOST_INPUT% --tcp-port %PORT_INPUT%

if errorlevel 1 (
    echo.
    echo ================================================================================
    echo ERROR: Connection failed
    echo ================================================================================
    echo.
    echo Troubleshooting:
    echo   1. Verify Spresense IP address (check serial console)
    echo   2. Ensure PC and Spresense are on the same WiFi network
    echo   3. Check Windows Firewall settings
    echo   4. Try: ping %HOST_INPUT%
    echo.
    pause
) else (
    echo.
    echo Connection closed.
    echo.
    pause
)
