#!/bin/bash
# Spresense Security Camera GUI Viewer
# Usage: ./run_gui.sh

cd "$(dirname "$0")"

# Check if binary exists
if [ ! -f "target/release/security_camera_gui" ]; then
    echo "Building GUI application..."
    cargo build --release --features gui --bin security_camera_gui
fi

# Force X11 backend (required for WSL2)
export WINIT_UNIX_BACKEND=x11

# Disable Wayland
export WAYLAND_DISPLAY=

# Set DISPLAY for WSL2
# For WSLg (Windows 11): use :0
# For older WSL2 with VcXsrv: use Windows IP
if [ -z "$DISPLAY" ]; then
    # Try WSLg first (Windows 11)
    export DISPLAY=:0

    # If that doesn't work, try VcXsrv method
    # export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0
fi

# Use software rendering for WSL2 (no GPU acceleration)
export LIBGL_ALWAYS_SOFTWARE=1
export MESA_GL_VERSION_OVERRIDE=3.3
export MESA_GLSL_VERSION_OVERRIDE=330
export GALLIUM_DRIVER=llvmpipe
export LIBGL_ALWAYS_INDIRECT=1
export __GLX_VENDOR_LIBRARY_NAME=mesa

# Check if X11 is available
echo "Checking X11 connection..."
if ! xdpyinfo &>/dev/null; then
    echo ""
    echo "⚠️  X11 server not detected!"
    echo ""
    echo "Please start an X11 server on Windows:"
    echo "  1. Windows 11: WSLg is built-in (should work automatically)"
    echo "  2. Windows 10: Install VcXsrv or X410"
    echo "     - VcXsrv: https://sourceforge.net/projects/vcxsrv/"
    echo "     - Start with: 'XLaunch' → Multiple Windows → Display 0"
    echo ""
    echo "Current DISPLAY: $DISPLAY"
    echo ""
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Run GUI
echo ""
echo "Starting Spresense Security Camera GUI..."
echo "DISPLAY: $DISPLAY"
echo "Note: Using software rendering (may be slower in WSL2)"
echo ""
./target/release/security_camera_gui
