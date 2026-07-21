#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# dora-test-utils — Week 8 Demo Script
# ─────────────────────────────────────────────────────────────
# Showcases Week 8 additions:
#   1. Echo pipeline (backward compat — Week 6)
#   2. Multi-output echo pipeline (NEW — multi-output test-source)
#   3. Classifier pipeline (NEW — classifier-node with threshold split)
#   4. Integration tests (4 Week 6 + 2 Week 8)
#   5. Library unit tests
# ─────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

banner() {
    echo ""
    echo -e "${CYAN}${BOLD}═══ $1 ═══${NC}"
    echo ""
}

step() {
    echo -e "${GREEN}▶ $1${NC}"
}

warn() {
    echo -e "${RED}⚠ $1${NC}"
}

# ─── 0. Prerequisites ────────────────────────────────────
banner "0. Check prerequisites"

step "Check Rust toolchain..."
cargo --version
rustc --version

# ─── 1. Build all binaries ───────────────────────────────
banner "1. Build all binaries"

BUILD_LOG=$(mktemp)
trap "rm -f $BUILD_LOG" EXIT

step "Build test-source, test-sink, echo-node, classifier-node..."
if cargo build --bin test-source --bin test-sink --bin echo-node --bin classifier-node > "$BUILD_LOG" 2>&1; then
    tail -1 "$BUILD_LOG"
else
    warn "Build failed! Last 20 lines:"
    tail -20 "$BUILD_LOG"
    exit 1
fi

step "Build dora CLI..."
if PYO3_NO_PYTHON=1 cargo build --manifest-path dora/Cargo.toml -p dora-cli > "$BUILD_LOG" 2>&1; then
    tail -1 "$BUILD_LOG"
else
    warn "dora CLI build failed! Last 20 lines:"
    tail -20 "$BUILD_LOG"
    exit 1
fi

DORA="dora/target/debug/dora"

# ─── 2. Echo Pipeline (backward compat) ──────────────────
banner "2. Echo Pipeline (backward-compatible single output)"

echo "Dataflow: tests/fixtures/echo-dataflow.yml"
echo ""
echo "Architecture:"
echo "  test-source --data--> echo-node --data--> test-sink → result.json"
echo ""
cat tests/fixtures/echo-dataflow.yml

banner "3. Echo: Test Data"

echo "source-data.json → test-source input:"
cat tests/fixtures/source-data.json
echo ""
echo "expected-output.json → test-sink expectations:"
cat tests/fixtures/expected-output.json

banner "4. Echo: Run Pipeline"

echo "Starting dora run (--stop-after 10s)..."
echo ""
DORA_LOG=$(mktemp)
set +e
$DORA run tests/fixtures/echo-dataflow.yml --stop-after 10s > "$DORA_LOG" 2>&1
DORA_EXIT=$?
set -e
grep -E "(spawning|ready|finished|match|error)" "$DORA_LOG" || true
rm -f "$DORA_LOG"
if [ $DORA_EXIT -ne 0 ]; then
    warn "dora run exited with code $DORA_EXIT"
fi
echo ""

banner "5. Echo: Result"

RESULT_FILE=""
if [ -f tests/fixtures/result.json ]; then
    RESULT_FILE="tests/fixtures/result.json"
elif [ -f result.json ]; then
    RESULT_FILE="result.json"
fi

if [ -n "$RESULT_FILE" ]; then
    cat "$RESULT_FILE"
    echo ""
    if grep -q '"match": true' "$RESULT_FILE"; then
        echo -e "${GREEN}${BOLD}✅ Echo pipeline PASSED — all 3 values matched!${NC}"
    else
        echo -e "${RED}${BOLD}❌ Echo pipeline FAILED${NC}"
    fi
else
    warn "result.json not found (checked tests/fixtures/ and ./)"
fi

# ─── 6. Multi-Output Echo Pipeline (NEW) ─────────────────
banner "6. Multi-Output Echo Pipeline (NEW — Week 8)"

echo "Architecture:"
echo "  test-source --data_a--> echo-a --data_a--> test-sink-a → result-a.json"
echo "               --data_b--> echo-b --data_b--> test-sink-b → result-b.json"
echo ""
echo "One test-source process emits on 2 outputs; each routes through"
echo "a dedicated echo node to a dedicated sink."
echo ""
echo "Dataflow: tests/fixtures/multi-echo-dataflow.yml"
echo ""
cat tests/fixtures/multi-echo-dataflow.yml

banner "7. Multi-Echo: Test Data"

echo "Output data_a (from source-data.json):"
cat tests/fixtures/source-data.json
echo ""
echo "Output data_b (from classifier-source.json):"
cat tests/fixtures/classifier-source.json
echo ""
echo "Expected for sink-a (expected-output.json):"
cat tests/fixtures/expected-output.json
echo ""
echo "Expected for sink-b (classifier-source-expected.json):"
cat tests/fixtures/classifier-source-expected.json

banner "8. Multi-Echo: Run Pipeline"

echo "Starting dora run (--stop-after 15s)..."
echo ""
DORA_LOG=$(mktemp)
set +e
$DORA run tests/fixtures/multi-echo-dataflow.yml --stop-after 15s > "$DORA_LOG" 2>&1
DORA_EXIT=$?
set -e
grep -E "(spawning|ready|finished|match|error)" "$DORA_LOG" || true
rm -f "$DORA_LOG"
if [ $DORA_EXIT -ne 0 ]; then
    warn "dora run exited with code $DORA_EXIT"
fi
echo ""

banner "9. Multi-Echo: Results"

PASSED=0
for F in result-a.json result-b.json; do
    RF=""
    if [ -f "tests/fixtures/$F" ]; then
        RF="tests/fixtures/$F"
    elif [ -f "$F" ]; then
        RF="$F"
    fi
    if [ -n "$RF" ]; then
        echo "$F:"
        cat "$RF"
        echo ""
        if grep -q '"match": true' "$RF"; then
            echo -e "${GREEN}  ✅ $F PASSED${NC}"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}  ❌ $F FAILED${NC}"
        fi
    else
        warn "$F not found"
    fi
    echo ""
done

if [ $PASSED -eq 2 ]; then
    echo -e "${GREEN}${BOLD}✅ Multi-echo pipeline PASSED — both outputs matched!${NC}"
else
    echo -e "${RED}${BOLD}❌ Multi-echo pipeline: $PASSED/2 passed${NC}"
fi

# ─── 10. Classifier Pipeline (NEW) ───────────────────────
banner "10. Classifier Pipeline (NEW — Week 8)"

echo "Architecture:"
echo "  test-source --raw-data--> classifier --high--> test-sink-high → result-high.json"
echo "                                       --low---> test-sink-low  → result-low.json"
echo ""
echo "classifier-node: receives Int64 values, splits by threshold 50."
echo "  val > 50  → \"high\" output"
echo "  val ≤ 50  → \"low\" output"
echo ""
echo "Dataflow: tests/fixtures/classifier-dataflow.yml"
echo ""
cat tests/fixtures/classifier-dataflow.yml

banner "11. Classifier: Test Data"

echo "classifier-source.json (9 values):"
cat tests/fixtures/classifier-source.json
echo ""
echo "classifier-expected-high.json (values > 50 → 4 elements):"
cat tests/fixtures/classifier-expected-high.json
echo ""
echo "classifier-expected-low.json (values ≤ 50 → 5 elements):"
cat tests/fixtures/classifier-expected-low.json

banner "12. Classifier: Run Pipeline"

echo "Starting dora run (--stop-after 15s)..."
echo ""
DORA_LOG=$(mktemp)
set +e
$DORA run tests/fixtures/classifier-dataflow.yml --stop-after 15s > "$DORA_LOG" 2>&1
DORA_EXIT=$?
set -e
grep -E "(spawning|ready|finished|match|error)" "$DORA_LOG" || true
rm -f "$DORA_LOG"
if [ $DORA_EXIT -ne 0 ]; then
    warn "dora run exited with code $DORA_EXIT"
fi
echo ""

banner "13. Classifier: Results"

PASSED=0
for F in result-high.json result-low.json; do
    RF=""
    if [ -f "tests/fixtures/$F" ]; then
        RF="tests/fixtures/$F"
    elif [ -f "$F" ]; then
        RF="$F"
    fi
    if [ -n "$RF" ]; then
        echo "$F:"
        cat "$RF"
        echo ""
        if grep -q '"match": true' "$RF"; then
            echo -e "${GREEN}  ✅ $F PASSED${NC}"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}  ❌ $F FAILED${NC}"
        fi
    else
        warn "$F not found"
    fi
    echo ""
done

if [ $PASSED -eq 2 ]; then
    echo -e "${GREEN}${BOLD}✅ Classifier pipeline PASSED — high/low both matched!${NC}"
else
    echo -e "${RED}${BOLD}❌ Classifier pipeline: $PASSED/2 passed${NC}"
fi

# ─── 14. Integration Tests ───────────────────────────────
banner "14. Integration Test Suite"

echo "Running 6 automated integration tests (4 echo + 2 Week 8)..."
echo ""
INTEGRATION_LOG=$(mktemp)
set +e
cargo test --test integration -- --test-threads=1 --nocapture > "$INTEGRATION_LOG" 2>&1
INTEGRATION_EXIT=$?
set -e

# Show each test result line
grep -E "(test .*::.*\.\.\. |test result)" "$INTEGRATION_LOG" || true
rm -f "$INTEGRATION_LOG"
echo ""

if [ $INTEGRATION_EXIT -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✅ All integration tests passed!${NC}"
else
    warn "Some integration tests failed (exit code $INTEGRATION_EXIT)"
fi

# ─── 15. Library Unit Tests ──────────────────────────────
banner "15. Library Unit Tests"

echo "(using --test-threads=1 to avoid flume 0.10 spinlock contention)"
echo ""

UNIT_LOG=$(mktemp)
set +e
timeout 30 cargo test --lib -- --test-threads=1 > "$UNIT_LOG" 2>&1
UNIT_EXIT=$?
set -e

UNIT_RESULT=$(grep "test result" "$UNIT_LOG" || true)
if [ $UNIT_EXIT -eq 124 ]; then
    warn "unit tests timed out after 30s"
elif [ $UNIT_EXIT -ne 0 ]; then
    warn "unit tests failed with exit code $UNIT_EXIT"
    echo "$UNIT_RESULT"
else
    echo "$UNIT_RESULT"
fi
rm -f "$UNIT_LOG"

# ─── Done ────────────────────────────────────────────────
banner "Week 8 Demo Complete"

echo -e "${GREEN}${BOLD}Summary:${NC}"
echo ""
echo "  Pipelines demonstrated:"
echo "    • Echo pipeline:          test-source → echo-node → test-sink        (backward compat)"
echo "    • Multi-echo pipeline:    test-source → 2×echo → 2×sink              (multi-output)"
echo "    • Classifier pipeline:    test-source → classifier → 2×sink           (threshold split)"
echo ""
echo "  Week 8 key additions:"
echo "    • Multi-output test-source:  --output ID:FILE.json (repeatable flag)"
echo "    • Classifier node:           Int64 threshold classification (env-configurable)"
echo "    • 2 new dataflow pipelines:  multi-echo (5-node), classifier (4-node)"
echo "    • 2 new integration tests:   multi_echo_pipeline_two_outputs, classifier_pipeline_basic"
echo ""
echo "  Test counts:"
echo "    • Library unit tests:  $(echo "$UNIT_RESULT" | grep -o '[0-9]\+ passed' || echo 'see above')"
echo "    • Integration tests:   6 (4 echo + 2 Week 8)"
echo "    • CI jobs:             5 (check, test, clippy, fmt, integration-test)"
echo ""
echo -e "${CYAN}Repo:${NC}   https://github.com/SunSunSun689/gsoc2026-dora-test-utils"
echo -e "${CYAN}Branch:${NC} week8"
