================================================================================
  Spresense Security Camera Viewer - Windows Release
================================================================================

Version: Phase 7.3.3 (Error Handling Mode)
Build Date: 2026-01-13
Platform: Windows x64

================================================================================
  Contents
================================================================================

1. security_camera_gui.exe       (16 MB)  - GUI version (Recommended)
2. security_camera_viewer.exe    (4.6 MB) - CLI version
3. README.txt                              - This file

================================================================================
  Quick Start
================================================================================

Method 1: GUI Version (Recommended)
------------------------------------

1. Double-click "security_camera_gui.exe"
2. Select transport type:
   - USB/Serial: COM port connection
   - TCP: WiFi network connection
3. Click "Start Capture"

Method 2: Command Line (Advanced)
----------------------------------

USB/Serial connection:
  security_camera_viewer.exe --port COM3 --baud 921600

TCP/WiFi connection:
  security_camera_viewer.exe --transport tcp --tcp-host 192.168.137.50 --tcp-port 8888

================================================================================
  Error Handling Mode (Phase 7.3.3 NEW!)
================================================================================

The GUI now includes two error handling modes for different use cases:

Production Mode (🟢 Default):
  - Continues operation even when errors occur
  - Ideal for continuous 24/7 operation
  - Logs errors but doesn't stop capture
  - Recommended for production deployments

Debug Mode (🔴):
  - Stops capture after 10 consecutive packet errors
  - Ideal for development and troubleshooting
  - Helps identify problems early
  - Recommended for debugging

How to switch:
1. Open GUI Settings panel (left side)
2. Look for "Error Handling:" section
3. Select "🟢 Production" or "🔴 Debug"
4. Restart capture for changes to take effect

Benefits:
  - Production Mode: Prevents Spresense from stopping when PC has issues
  - Enables long-term continuous operation
  - Improves product reliability

================================================================================
  WiFi/TCP Setup
================================================================================

1. Spresense Setup:
   - Flash WiFi-enabled firmware to Spresense
   - Configure WiFi SSID and password
   - Note the IP address displayed on serial console

2. PC Setup:
   - Connect PC to the same WiFi network
   - Run GUI or CLI with TCP transport option
   - Enter Spresense IP address and port (default: 8888)

3. Connection:
   GUI:
     Transport Type: TCP
     Host: 192.168.x.x (Spresense IP)
     Port: 8888

   CLI:
     security_camera_viewer.exe --transport tcp --tcp-host 192.168.x.x --tcp-port 8888

================================================================================
  Command Line Options
================================================================================

USB/Serial Mode:
  --port <COM_PORT>           Serial port (e.g., COM3)
  --baud <BAUD_RATE>          Baud rate (default: 921600)

TCP Mode:
  --transport tcp             Use TCP transport
  --tcp-host <IP_ADDRESS>     Spresense IP address
  --tcp-port <PORT>           TCP port (default: 8888)

Recording:
  --record <OUTPUT_DIR>       Save frames to directory
  --max-frames <COUNT>        Maximum frames to record

Display:
  --help                      Show help message

================================================================================
  Troubleshooting
================================================================================

Problem: "GUI window doesn't open"
Solution:
  - Check if antivirus software is blocking the executable
  - Run as Administrator
  - Check Windows Event Viewer for errors

Problem: "Cannot connect to Spresense (TCP)"
Solution:
  - Verify Spresense IP address (check serial console)
  - Ensure PC and Spresense are on the same WiFi network
  - Check Windows Firewall settings
  - Try: ping 192.168.x.x (Spresense IP)

Problem: "COM port not found"
Solution:
  - Check Device Manager for COM port number
  - Install Spresense USB drivers if needed
  - Reconnect Spresense board

Problem: "Connection timeout"
Solution:
  - Check WiFi signal strength
  - Verify WiFi credentials on Spresense
  - Increase timeout in settings

Problem: "TCP connection drops immediately (Error -107 on Spresense)"
Solution:
  1. Run debug_tcp.bat to see detailed logs
  2. Check for error messages in the console window
  3. Verify buffer size is sufficient (Phase 7.2a: 200KB)
  4. Ensure you're using the latest build (check timestamp)

================================================================================
  Debug Mode (Advanced)
================================================================================

To enable detailed logging for troubleshooting:

Method 1: Use debug_tcp.bat
  - Double-click "debug_tcp.bat"
  - This will show detailed connection logs
  - Useful for diagnosing TCP connection issues

Method 2: Manual command line
  - Open Command Prompt
  - Run: set RUST_LOG=debug
  - Run: security_camera_gui.exe
  - Check console output for error messages

Method 3: CLI with verbose logging
  - Run: set RUST_LOG=debug
  - Run: security_camera_viewer.exe --transport tcp --tcp-host 192.168.x.x -v

Log file location:
  - Console output only (no log file by default)
  - To save logs: security_camera_gui.exe > log.txt 2>&1

================================================================================
  Performance Tips
================================================================================

USB/Serial (921600 baud):
  - Expected FPS: 11-13 fps @ 320x240 JPEG
  - Stable, reliable connection
  - Cable required

WiFi/TCP:
  - Expected FPS: 15-25 fps @ 320x240 JPEG (depending on WiFi)
  - Wireless, flexible placement
  - WiFi signal quality affects performance

================================================================================
  Network Configuration
================================================================================

Current WiFi Settings (Spresense firmware):
  SSID:     DESKTOP-GPU979R
  Password: B54p3530
  TCP Port: 8888
  IP Mode:  DHCP (automatic)

To find Spresense IP address:
1. Connect to Spresense serial console
2. Run "security_camera" application
3. Look for: "WiFi connected! IP: 192.168.x.x"

================================================================================
  System Requirements
================================================================================

- Operating System: Windows 10/11 (64-bit)
- RAM: 512 MB minimum, 1 GB recommended
- Display: 1024x768 or higher
- Network: WiFi adapter (for TCP mode)
- USB: USB port (for serial mode)

================================================================================
  File Information
================================================================================

security_camera_gui.exe:
  - Type: GUI application (eframe/egui)
  - Size: 16 MB
  - Dependencies: None (statically linked)
  - Features: Visual interface, real-time preview, transport selection

security_camera_viewer.exe:
  - Type: Console application
  - Size: 4.6 MB
  - Dependencies: None (statically linked)
  - Features: Command-line interface, headless operation, scripting

================================================================================
  Support & Documentation
================================================================================

Project Repository:
  /home/ken/Spr_ws/GH_wk_test/

Documentation:
  - Phase 7 Specification: docs/security_camera/PHASE7_WIFI_TCP_SPEC.md
  - Test Results: docs/security_camera/04_test_results/
  - Case Studies: docs/case_study/

Build Information:
  - Rust toolchain: rustc (latest stable)
  - Target: x86_64-pc-windows-gnu
  - Build mode: Release (optimized)
  - Cross-compiled on: WSL2 Ubuntu

================================================================================
  License & Credits
================================================================================

Copyright 2025 Security Camera Project

Built with:
  - Rust programming language
  - egui/eframe (GUI framework)
  - serialport (Serial communication)
  - tokio (Async runtime)
  - image (Image processing)

Generated with Claude Code
https://claude.com/claude-code

================================================================================
  Version History
================================================================================

Phase 7.3.3 (2026-01-13):
  - Error handling mode (Production/Debug)
  - Continuous operation support (24/7 running)
  - GUI toggle switch for error handling mode
  - Prevents PC errors from stopping Spresense
  - Enhanced reliability for production deployments

Phase 7.3.2 (2026-01-12):
  - Accurate packet boundary handling
  - CRC 2-byte fix in TCP protocol
  - Buffer state management improvements

Phase 7.3 (2026-01-12):
  - Stateful TCP reading (150KB initial buffer)
  - One-time sync search optimization
  - 64KB incremental buffer refill

Phase 7 (2026-01-03):
  - WiFi/TCP transport support
  - GS2200M WiFi driver integration
  - Dual transport mode (USB/WiFi)
  - Mobile AP support
  - TCP performance metrics (send time measurement)
  - CSV logging with TCP statistics

Phase 4.2 (2025-12-31):
  - GUI viewer implementation
  - Real-time MJPEG display
  - Recording functionality

Phase 2 (2025-12-30):
  - Multi-threaded pipelining
  - Performance optimization (11.05 fps average)
  - 99.895% success rate

================================================================================
End of README
================================================================================
