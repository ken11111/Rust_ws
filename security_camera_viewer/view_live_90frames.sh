#!/bin/bash
# Live viewer for 90-frame limited mode
# Start this FIRST, then start Spresense app

TEMP_DIR=$(mktemp -d)
OUTPUT_DIR="$TEMP_DIR/frames"

echo "==================================================="
echo "Spresense Live Camera Viewer (90-frame mode)"
echo "==================================================="
echo "Output: $OUTPUT_DIR"
echo ""
echo "📌 IMPORTANT: Follow these steps:"
echo ""
echo "1. This script is now WAITING for data..."
echo "2. Go to Spresense console and run:"
echo "     nsh> security_camera"
echo "3. Viewer will start automatically when frames arrive"
echo ""
echo "Press Ctrl+C to cancel"
echo "==================================================="

# Cleanup on exit
trap "rm -rf $TEMP_DIR; echo 'Stopped'" EXIT

# Start capturing in background with longer timeout
./target/release/security_camera_viewer \
    --individual-files \
    --output "$OUTPUT_DIR" \
    --max-frames 90 \
    --max-errors 30 &

CAPTURE_PID=$!

# Wait for first frame with progress indicator
echo ""
echo "Waiting for Spresense to start sending..."
MAX_WAIT=60
waited=0
while [ $waited -lt $MAX_WAIT ]; do
    frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
    if [ $frame_count -gt 0 ]; then
        echo ""
        echo "✓ First frame received! ($frame_count frames so far)"
        break
    fi
    sleep 1
    waited=$((waited + 1))

    # Progress indicator
    if [ $((waited % 5)) -eq 0 ]; then
        echo "  Waiting... ${waited}s (timeout in $((MAX_WAIT - waited))s)"
    fi
done

# Check if we got any frames
frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
if [ $frame_count -eq 0 ]; then
    echo ""
    echo "❌ Error: No frames received after ${MAX_WAIT}s"
    echo ""
    echo "Did you start the Spresense app?"
    kill $CAPTURE_PID 2>/dev/null
    exit 1
fi

# Start viewer
if command -v feh &> /dev/null; then
    echo "Starting feh viewer (fullscreen)..."
    echo "Capturing remaining frames..."
    feh --reload 0.3 --auto-zoom --fullscreen "$OUTPUT_DIR" &
    VIEWER_PID=$!
else
    echo "Warning: feh not found, frames saved to: $OUTPUT_DIR"
fi

# Monitor capture progress
echo ""
echo "==================================================="
prev_count=0
while kill -0 $CAPTURE_PID 2>/dev/null; do
    frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
    if [ $frame_count -ne $prev_count ]; then
        echo "Frames captured: $frame_count / 90"
        prev_count=$frame_count
    fi
    sleep 1
done

wait $CAPTURE_PID

# Final count
frame_count=$(ls -1 "$OUTPUT_DIR"/*.jpg 2>/dev/null | wc -l)
echo "==================================================="
echo "✓ Capture complete: $frame_count frames"
echo "==================================================="
echo ""
echo "Frames saved to: $OUTPUT_DIR"
echo "Press 'q' in feh to exit viewer"
echo ""

# Keep viewer running
if [ -n "$VIEWER_PID" ]; then
    wait $VIEWER_PID
fi
