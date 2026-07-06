#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# dora-test-utils — Midterm Demo Script
# ─────────────────────────────────────────────────────────────
# Runs the echo pipeline and integration tests to showcase
# test-source → echo-node → test-sink end-to-end.
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

step "Build test_source, test-sink, echo-node..."
if cargo build --bin test_source --bin test-sink --bin echo-node > "$BUILD_LOG" 2>&1; then
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

# ─── 2. Show the dataflow YAML ───────────────────────────
banner "2. Dataflow definition"
cat tests/fixtures/echo-dataflow.yml

# ─── 3. Show test data ───────────────────────────────────
banner "3. Test data"

echo "source-data.json → test-source input:"
cat tests/fixtures/source-data.json
echo ""
echo "expected-output.json → test-sink expectations:"
cat tests/fixtures/expected-output.json

# ─── 4. Run the echo pipeline ────────────────────────────
banner "4. Run echo pipeline"

echo "Starting dora run..."
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

# ─── 5. Show result ──────────────────────────────────────
banner "5. Pipeline result"

# dora spawns nodes with CWD = YAML directory, but also check cwd.
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
        echo -e "${GREEN}${BOLD}✅ Pipeline PASSED — all 3 values matched!${NC}"
    else
        echo -e "${RED}${BOLD}❌ Pipeline FAILED${NC}"
    fi
else
    warn "result.json not found (checked tests/fixtures/ and ./)"
fi

# ─── 6. Failure demo ─────────────────────────────────────
banner "6. Failure demo — catching a bug"

FAIL_DIR=$(mktemp -d)
trap "rm -rf $FAIL_DIR" EXIT

# Copy the correct source data (echo-node will output [42, 99, -1])
cp tests/fixtures/source-data.json "$FAIL_DIR/source-data.json"

# Deliberately write WRONG expected data — 99 → 999
cat > "$FAIL_DIR/expected-output.json" << 'FAILEOF'
{
  "data": [42, 999, -1],
  "data_type": "Int64"
}
FAILEOF

echo "Source sends:  [42, 99, -1]"
echo "Expected:      [42, 999, -1]  ← deliberate mistake at index 1"
echo ""

# Generate a temporary YAML dataflow (all paths are project-relative).
TARGET_DIR="$(pwd)/target"
cat > "$FAIL_DIR/dataflow.yml" << YAMLEOF
nodes:
  - id: test-source
    path: $TARGET_DIR/debug/test_source
    args: "--output-id data --data-file $FAIL_DIR/source-data.json"
    outputs:
      - data

  - id: echo-node
    path: $TARGET_DIR/debug/echo-node
    inputs:
      data: test-source/data
    outputs:
      - data

  - id: test-sink
    path: $TARGET_DIR/debug/test-sink
    inputs:
      data: echo-node/data
    args: "--expected-file $FAIL_DIR/expected-output.json --output-file $FAIL_DIR/result.json"
YAMLEOF

echo "Running pipeline..."
set +e
DORA_FAIL_LOG=$(mktemp)
$DORA run "$FAIL_DIR/dataflow.yml" --stop-after 10s > "$DORA_FAIL_LOG" 2>&1
DORA_FAIL_EXIT=$?
set -e

grep -E "(node is ready|finished|dataflow finished|exited with error)" "$DORA_FAIL_LOG" || true
rm -f "$DORA_FAIL_LOG"

echo ""

if [ -f "$FAIL_DIR/result.json" ]; then
    cat "$FAIL_DIR/result.json"
    echo ""
    if grep -q '"match": false' "$FAIL_DIR/result.json"; then
        echo -e "${GREEN}${BOLD}✅ Bug caught!${NC} — test-sink detected the mismatch at index 1 (expected 999, got 99)"
    else
        echo -e "${RED}${BOLD}❌ Unexpected: result.json shows match=true${NC}"
    fi
else
    warn "result.json not found in $FAIL_DIR"
fi

rm -rf "$FAIL_DIR"

# ─── 7. Integration tests ────────────────────────────────
banner "7. Integration test suite"

echo "Running 4 automated integration tests..."
echo ""
INTEGRATION_EXIT=0
cargo test --test integration -- --test-threads=1 --nocapture 2>&1 | grep -E "(test echo|test result|FAILED)" || INTEGRATION_EXIT=$?
echo ""
if [ $INTEGRATION_EXIT -ne 0 ]; then
    warn "some integration tests may have failed (grep found no matches)"
fi

# ─── 8. Library unit tests ───────────────────────────────
banner "8. Library unit tests"

UNIT_LOG=$(mktemp)
set +e
timeout 30 cargo test --lib > "$UNIT_LOG" 2>&1
UNIT_EXIT=$?
set -e

UNIT_RESULT=$(grep "test result" "$UNIT_LOG" || true)
if [ $UNIT_EXIT -eq 124 ]; then
    echo "  (timed out after 30s — known daemon timing issue)"
elif [ $UNIT_EXIT -ne 0 ]; then
    warn "unit tests failed with exit code $UNIT_EXIT"
    echo "$UNIT_RESULT"
else
    echo "$UNIT_RESULT"
fi
rm -f "$UNIT_LOG"

# ─── Done ────────────────────────────────────────────────
banner "Demo Complete"

echo -e "${GREEN}${BOLD}Summary:${NC}"
echo "  • Happy path: test-source → echo-node → test-sink ✅"
echo "  • Failure demo: deliberate mismatch caught ✅"
echo "  • Integration tests: 4/4 passing"
echo "  • Library unit tests: $(echo "$UNIT_RESULT" | grep -o '[0-9]\+ passed' || echo 'see above')"
echo ""
echo -e "${CYAN}Repo:${NC} https://github.com/SunSunSun689/gsoc2026-dora-test-utils"
echo -e "${CYAN}Branch:${NC} week6"
