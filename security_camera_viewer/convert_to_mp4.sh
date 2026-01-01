#!/bin/bash
# MJPEG to MP4 converter
# Usage: ./convert_to_mp4.sh [input.mjpeg] [output.mp4]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "MJPEG to MP4 Converter"
echo "========================================="
echo ""

# Check if ffmpeg is installed
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}Error: ffmpeg is not installed${NC}"
    echo ""
    echo "Please install ffmpeg:"
    echo "  Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo "  Windows: Download from https://ffmpeg.org/download.html"
    echo ""
    exit 1
fi

# Check arguments
if [ $# -eq 0 ]; then
    echo "Usage: $0 <input.mjpeg> [output.mp4]"
    echo ""
    echo "Examples:"
    echo "  $0 recording_20260101_123456.mjpeg"
    echo "  $0 recording_20260101_123456.mjpeg output.mp4"
    echo "  $0 recordings/*.mjpeg  # Convert all MJPEG files"
    echo ""
    exit 1
fi

# Function to convert single file
convert_file() {
    local input="$1"
    local output="$2"

    # Check input file exists
    if [ ! -f "$input" ]; then
        echo -e "${RED}Error: Input file not found: $input${NC}"
        return 1
    fi

    # Generate output filename if not provided
    if [ -z "$output" ]; then
        output="${input%.mjpeg}.mp4"
    fi

    # Get input file size
    local input_size=$(stat -f%z "$input" 2>/dev/null || stat -c%s "$input" 2>/dev/null)
    local input_size_mb=$(echo "scale=2; $input_size / 1048576" | bc)

    echo -e "${YELLOW}Converting:${NC} $input"
    echo -e "${YELLOW}Output:${NC} $output"
    echo -e "${YELLOW}Input size:${NC} ${input_size_mb} MB"
    echo ""

    # Convert MJPEG to MP4
    # -i: input file
    # -c:v libx264: use H.264 codec (widely supported)
    # -preset medium: encoding speed/quality balance
    # -crf 23: constant quality (18-28, lower = better quality)
    # -movflags +faststart: optimize for web streaming
    # -y: overwrite output file

    if ffmpeg -i "$input" \
        -c:v libx264 \
        -preset medium \
        -crf 23 \
        -movflags +faststart \
        -y \
        "$output" 2>&1 | grep -E "frame=|Duration:|bitrate:|video:"; then

        # Get output file size
        local output_size=$(stat -f%z "$output" 2>/dev/null || stat -c%s "$output" 2>/dev/null)
        local output_size_mb=$(echo "scale=2; $output_size / 1048576" | bc)
        local ratio=$(echo "scale=1; ($output_size * 100) / $input_size" | bc)

        echo ""
        echo -e "${GREEN}✓ Conversion successful!${NC}"
        echo -e "  Input:  ${input_size_mb} MB"
        echo -e "  Output: ${output_size_mb} MB (${ratio}% of original)"
        echo -e "  Saved:  $output"
        echo ""
        return 0
    else
        echo ""
        echo -e "${RED}✗ Conversion failed${NC}"
        echo ""
        return 1
    fi
}

# Convert files
success_count=0
fail_count=0

for input in "$@"; do
    if [ -f "$input" ]; then
        if convert_file "$input"; then
            ((success_count++))
        else
            ((fail_count++))
        fi
    else
        echo -e "${YELLOW}Warning: Skipping non-file argument: $input${NC}"
        echo ""
    fi
done

# Summary
echo "========================================="
echo "Conversion Summary"
echo "========================================="
echo -e "Success: ${GREEN}$success_count${NC}"
echo -e "Failed:  ${RED}$fail_count${NC}"
echo ""

if [ $success_count -gt 0 ]; then
    echo -e "${GREEN}MP4 files are ready for playback!${NC}"
    echo ""
    echo "You can play them with:"
    echo "  - VLC Media Player"
    echo "  - Windows Media Player"
    echo "  - Any modern video player"
    echo ""
fi

exit 0
