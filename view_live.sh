#!/bin/bash
# Simple MJPEG viewer for WSL2
# Captures frames and displays them in real-time using feh/eog

TEMP_DIR=$(mktemp -d)
OUTPUT_DIR="$TEMP_DIR/frames"
MAX_WAIT=30

echo "==================================================="
echo "Spresense Live Camera Viewer"
echo "==================================================="
echo "Output: $OUTPUT_DIR"
echo ""
echo "Make sure Spresense security_camera app is running!"
echo "  nsh> sercon"
echo "  nsh> security_camera"
echo ""
echo "Press Ctrl+C to stop"
echo "==================================================="

# Cleanup on exit
trap "rm -rf $TEMP_DIR; echo 'Stopped'" EXIT

# Start capturing in background
./target/release/security_camera_viewer \
    --individual-files \
    --output "$OUTPUT_DIR" \
    --max-frames 300 &

CAPTURE_PID=$!

# Wait for first frame
echo "Waiting for frames (timeout: ${MAX_WAIT}s)..."
waited=0
while [ $waited -lt $MAX_WAIT ]; do
    frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
    if [ $frame_count -gt 0 ]; then
        echo "✓ First frame received!"
        break
    fi
    sleep 1
    waited=$((waited + 1))
    if [ $((waited % 5)) -eq 0 ]; then
        echo "  Still waiting... (${waited}s)"
    fi
done

# Check if we got any frames
frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
if [ $frame_count -eq 0 ]; then
    echo ""
    echo "❌ Error: No frames received after ${MAX_WAIT}s"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Is Spresense connected? (check /dev/ttyACM0)"
    echo "  2. Is security_camera app running on Spresense?"
    echo "  3. Check logs above for connection errors"
    kill $CAPTURE_PID 2>/dev/null
    exit 1
fi

# Check if feh or eog is available
if command -v feh &> /dev/null; then
    echo "Starting feh viewer (auto-reload every 0.5s)..."
    feh --reload 0.5 --auto-zoom --fullscreen "$OUTPUT_DIR" &
    VIEWER_PID=$!
elif command -v eog &> /dev/null; then
    echo "Starting eog viewer..."
    eog "$OUTPUT_DIR" &
    VIEWER_PID=$!
else
    echo "Error: Neither 'feh' nor 'eog' found"
    echo "Install with: sudo apt-get install feh"
    kill $CAPTURE_PID
    exit 1
fi

echo ""
echo "==================================================="
echo "Viewer started! Frames: $OUTPUT_DIR"
echo "Press Ctrl+C to stop"
echo "==================================================="

# Wait for capture to finish
wait $CAPTURE_PID

echo "Capture finished"
