@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

REM MJPEG to MP4 converter for Windows
REM Usage: convert_to_mp4.bat [input.mjpeg] [output.mp4]

echo =========================================
echo MJPEG to MP4 Converter
echo =========================================
echo.

REM Check if ffmpeg is installed
where ffmpeg >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo Error: ffmpeg is not installed
    echo.
    echo Please install ffmpeg:
    echo   1. Download from https://ffmpeg.org/download.html
    echo   2. Extract to C:\ffmpeg
    echo   3. Add C:\ffmpeg\bin to PATH
    echo.
    echo Or install via Chocolatey:
    echo   choco install ffmpeg
    echo.
    pause
    exit /b 1
)

REM Check arguments
if "%~1"=="" (
    echo Usage: %~nx0 ^<input.mjpeg^> [output.mp4]
    echo.
    echo Examples:
    echo   %~nx0 recording_20260101_123456.mjpeg
    echo   %~nx0 recording_20260101_123456.mjpeg output.mp4
    echo   %~nx0 recordings\*.mjpeg
    echo.
    pause
    exit /b 1
)

set SUCCESS_COUNT=0
set FAIL_COUNT=0

REM Process all input files
for %%F in (%*) do (
    if exist "%%F" (
        call :convert_file "%%F"
    ) else (
        echo Warning: File not found: %%F
        echo.
    )
)

REM Summary
echo =========================================
echo Conversion Summary
echo =========================================
echo Success: %SUCCESS_COUNT%
echo Failed:  %FAIL_COUNT%
echo.

if %SUCCESS_COUNT% GTR 0 (
    echo MP4 files are ready for playback!
    echo.
    echo You can play them with:
    echo   - VLC Media Player
    echo   - Windows Media Player
    echo   - Any modern video player
    echo.
)

pause
exit /b 0

:convert_file
setlocal
set INPUT=%~1
set OUTPUT=%~dpn1.mp4

REM Allow custom output name if provided
if not "%~2"=="" set OUTPUT=%~2

echo Converting: %INPUT%
echo Output:     %OUTPUT%
echo.

REM Convert MJPEG to MP4
ffmpeg -i "%INPUT%" -c:v libx264 -preset medium -crf 23 -movflags +faststart -y "%OUTPUT%" 2>&1 | findstr /R /C:"frame=" /C:"Duration:" /C:"bitrate:" /C:"video:"

if %ERRORLEVEL% EQU 0 (
    echo.
    echo Conversion successful!
    echo Saved: %OUTPUT%
    echo.
    set /a SUCCESS_COUNT+=1
) else (
    echo.
    echo Conversion failed
    echo.
    set /a FAIL_COUNT+=1
)

endlocal & set SUCCESS_COUNT=%SUCCESS_COUNT%& set FAIL_COUNT=%FAIL_COUNT%
goto :eof
