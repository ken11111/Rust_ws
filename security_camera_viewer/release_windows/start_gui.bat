@echo off
REM Spresense Security Camera Viewer - GUI Launcher
REM Version: Phase 7 (WiFi/TCP Support)

echo ================================================================================
echo   Spresense Security Camera Viewer - GUI Mode
echo ================================================================================
echo.
echo Starting GUI application...
echo.

security_camera_gui.exe

if errorlevel 1 (
    echo.
    echo ================================================================================
    echo ERROR: Application exited with error code %errorlevel%
    echo ================================================================================
    echo.
    echo Troubleshooting:
    echo   1. Check if antivirus is blocking the application
    echo   2. Try running as Administrator
    echo   3. Check Windows Event Viewer for details
    echo.
    pause
)
