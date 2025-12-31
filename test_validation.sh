#!/bin/bash
# Phase 4.2 Re-test Validation Script
# 使用法: ./test_validation.sh metrics/metrics_YYYYMMDD_HHMMSS.csv

CSV_FILE="$1"

if [ -z "$CSV_FILE" ] || [ ! -f "$CSV_FILE" ]; then
    echo "Usage: $0 <csv_file>"
    echo "Example: $0 metrics/metrics_20251231_143022.csv"
    exit 1
fi

echo "==================================="
echo "Phase 4.2 Re-test Validation Report"
echo "==================================="
echo ""
echo "CSV File: $CSV_FILE"
echo ""

# 1. Data row count
ROWS=$(awk 'END {print NR-1}' "$CSV_FILE")
echo "1. CSV Data Rows: $ROWS"
echo "   Expected: ~1800 rows (30 min × 60 sec)"
echo ""

# 2. Average PC FPS
PC_FPS=$(awk -F',' 'NR>1 {sum+=$2; count++} END {printf "%.2f", sum/count}' "$CSV_FILE")
echo "2. Average PC FPS: $PC_FPS fps"
echo "   Expected: 19.0-20.5 fps"
echo "   Previous (buggy): 14.87 fps"

if (( $(echo "$PC_FPS >= 19.0" | bc -l) )); then
    echo "   ✅ PASS: FPS recovered to normal range"
else
    echo "   ❌ FAIL: FPS still below target"
fi
echo ""

# 3. Final error count
ERROR_COUNT=$(awk -F',' 'END {print $4}' "$CSV_FILE")
echo "3. Cumulative Errors: $ERROR_COUNT"
echo "   Expected: < 10 errors"
echo "   Previous (buggy): 114 errors"

if [ "$ERROR_COUNT" -lt 10 ]; then
    echo "   ✅ PASS: Error count within acceptable range"
else
    echo "   ❌ FAIL: Too many errors"
fi
echo ""

# 4. Decode failure rate (decode_time_ms = 0.00)
DECODE_FAILS=$(awk -F',' 'NR>1 && $5 == 0.00 {count++} END {print count+0}' "$CSV_FILE")
FAIL_RATE=$(awk -v fails="$DECODE_FAILS" -v total="$ROWS" 'BEGIN {printf "%.1f", (fails/total)*100}')
echo "4. Decode Failures: $DECODE_FAILS / $ROWS ($FAIL_RATE%)"
echo "   Expected: < 1%"
echo "   Previous (buggy): 43.5%"

if (( $(echo "$FAIL_RATE < 1.0" | bc -l) )); then
    echo "   ✅ PASS: Decode failure rate dramatically improved"
else
    echo "   ❌ FAIL: Still experiencing significant decode failures"
fi
echo ""

# 5. Average Serial read time
SERIAL_TIME=$(awk -F',' 'NR>1 {sum+=$6; count++} END {printf "%.2f", sum/count}' "$CSV_FILE")
echo "5. Average Serial Time: $SERIAL_TIME ms"
echo "   Expected: 48-50 ms"
echo "   Previous (buggy): 68.27 ms"

if (( $(echo "$SERIAL_TIME <= 55.0" | bc -l) )); then
    echo "   ✅ PASS: Serial read time improved"
else
    echo "   ⚠️  WARNING: Serial time still elevated"
fi
echo ""

# 6. Average JPEG size
JPEG_SIZE=$(awk -F',' 'NR>1 {sum+=$8; count++} END {printf "%.2f", sum/count}' "$CSV_FILE")
echo "6. Average JPEG Size: $JPEG_SIZE KB"
echo "   Expected: ~53 KB"
echo ""

# Summary
echo "==================================="
echo "Overall Assessment:"
echo "==================================="

PASS_COUNT=0
if (( $(echo "$PC_FPS >= 19.0" | bc -l) )); then ((PASS_COUNT++)); fi
if [ "$ERROR_COUNT" -lt 10 ]; then ((PASS_COUNT++)); fi
if (( $(echo "$FAIL_RATE < 1.0" | bc -l) )); then ((PASS_COUNT++)); fi
if (( $(echo "$SERIAL_TIME <= 55.0" | bc -l) )); then ((PASS_COUNT++)); fi

if [ "$PASS_COUNT" -eq 4 ]; then
    echo "🎉 ALL TESTS PASSED - Bug fix successful!"
    echo ""
    echo "Performance Recovery:"
    echo "  - PC FPS: 14.87 fps → $PC_FPS fps ($(awk -v old=14.87 -v new="$PC_FPS" 'BEGIN {printf "+%.1f%%", ((new-old)/old)*100}')"
    echo "  - Decode failures: 43.5% → $FAIL_RATE%"
    echo "  - Errors: 114 → $ERROR_COUNT"
    echo ""
    echo "✅ Ready to proceed to Phase 4.3: Error Recovery Test"
elif [ "$PASS_COUNT" -ge 2 ]; then
    echo "⚠️  PARTIAL SUCCESS - Some metrics improved, others need investigation"
    echo "   Passed: $PASS_COUNT / 4 tests"
else
    echo "❌ TESTS FAILED - Bug fix did not resolve the issues"
    echo "   Passed: $PASS_COUNT / 4 tests"
    echo ""
    echo "Please check:"
    echo "  1. Spresense device is running correctly"
    echo "  2. USB cable quality"
    echo "  3. Phase 4.2 code was properly built and deployed"
fi
echo "==================================="
