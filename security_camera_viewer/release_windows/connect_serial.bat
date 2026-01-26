@echo off
REM Spresense Security Camera Viewer - Serial Connection Script
REM Version: Phase 7 (WiFi/TCP Support)

echo ================================================================================
echo   Spresense Security Camera Viewer - Serial Mode
echo ================================================================================
echo.

REM Default settings
set DEFAULT_PORT=COM3
set DEFAULT_BAUD=921600

REM Prompt for COM port
echo Enter COM port (default: %DEFAULT_PORT%):
echo   Check Device Manager to find your Spresense COM port
set /p PORT_INPUT=
if "%PORT_INPUT%"=="" set PORT_INPUT=%DEFAULT_PORT%

echo Enter baud rate (default: %DEFAULT_BAUD%):
set /p BAUD_INPUT=
if "%BAUD_INPUT%"=="" set BAUD_INPUT=%DEFAULT_BAUD%

echo.
echo ================================================================================
echo Connecting to Spresense...
echo   Port: %PORT_INPUT%
echo   Baud: %BAUD_INPUT%
echo ================================================================================
echo.

REM Launch viewer
security_camera_viewer.exe --port %PORT_INPUT% --baud %BAUD_INPUT%

if errorlevel 1 (
    echo.
    echo ================================================================================
    echo ERROR: Connection failed
    echo ================================================================================
    echo.
    echo Troubleshooting:
    echo   1. Check Device Manager for correct COM port
    echo   2. Ensure Spresense is connected via USB
    echo   3. Install Spresense USB drivers if needed
    echo   4. Try a different COM port
    echo   5. Close other programs using the COM port
    echo.
    pause
) else (
    echo.
    echo Connection closed.
    echo.
    pause
)
