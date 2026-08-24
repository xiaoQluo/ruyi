#!/usr/bin/env bash
#
# run_examples.sh — Compile, run, and verify all Ruyi examples.
#
# Usage:
#   ./run_examples.sh              Compile and run all examples
#   ./run_examples.sh --verify     Recompile + run + compare against .expected baselines
#   ./run_examples.sh --update     Recompile + run + overwrite .expected baselines
#   ./run_examples.sh --only PAT   Only process examples matching PAT (glob)
#   ./run_examples.sh --help       Show this help message
#
set -euo pipefail

# ── Constants ────────────────────────────────────────────────────────────────
COMPILER="./target/release/ruyic"
EXAMPLES_DIR="./examples"
TARGET_DIR="./examples/target"
BASELINES_FILE="$TARGET_DIR/baselines.json"
FAILURES_LOG="$TARGET_DIR/failures.log"
COMPILE_TIMEOUT=60
RUN_TIMEOUT=10

# ── Examples coverage ───────────────────────────────────────────────────────
# Examples are auto-discovered recursively from $EXAMPLES_DIR via find;
# no explicit allowlist is required. New .ry files added to any subdirectory
# will be picked up on the next run.
#
# Directory structure:
#   basics/       — 语言基础 + 自动加载模块 (error/collections/arrays)
#   types/        — 类型系统 (generics, traits, pattern matching)
#   oop/          — 面向对象 (classes, objects, accessors)
#   concurrency/  — 并发与异步 (async, thread, channel, fiber)
#   stdlib/       — 需 import 的标准库模块演示
#   comprehensive/ — 综合演示

# ── Detect timeout command (macOS uses gtimeout from coreutils) ───────────────
if command -v gtimeout &>/dev/null; then
  TIMEOUT_CMD="gtimeout"
elif command -v timeout &>/dev/null; then
  TIMEOUT_CMD="timeout"
else
  TIMEOUT_CMD=""
fi

run_with_timeout() {
  local secs="$1"; shift
  if [[ -n "$TIMEOUT_CMD" ]]; then
    "$TIMEOUT_CMD" "$secs" "$@"
  else
    "$@"
  fi
}

# ── Counters ─────────────────────────────────────────────────────────────────
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0
FLAKY=0
EXP_FAILED=0
COMPILATION_FAILURES=0

# ── Per-file results (for final report) ──────────────────────────────────────
declare -a RESULTS=()
TMPDIR_CLEANUP=""

# ── Argument Parsing ─────────────────────────────────────────────────────────
MODE="default"
ONLY_PATTERN=""

usage() {
  cat <<'EOF'
Usage: run_examples.sh [OPTIONS]

Compile and run all Ruyi examples, then output a summary report.

Options:
  --verify     Recompile + run + compare output against .expected baselines
  --update     Recompile + run + overwrite .expected baselines with current output
  --only PAT   Only process examples matching the given pattern (glob)
  -h, --help   Show this help message and exit

Modes:
  default      Compile and run all examples, create/update baselines
  --verify     Verify examples against expected output files
  --update     Update expected output files with current results

Examples:
  ./run_examples.sh
  ./run_examples.sh --verify
  ./run_examples.sh --update
  ./run_examples.sh --only "hello"
  ./run_examples.sh --only "*async*"
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --verify)
      MODE="verify"
      shift
      ;;
    --update)
      MODE="update"
      shift
      ;;
    --only)
      if [[ -z "${2:-}" ]]; then
        echo "Error: --only requires a pattern argument" >&2
        exit 1
      fi
      ONLY_PATTERN="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Error: Unknown option '$1'" >&2
      usage >&2
      exit 1
      ;;
  esac
done

# ── Helper Functions ─────────────────────────────────────────────────────────

# Check if a file is a test file (needs --test flag).
# Arguments: $1 = basename (without .ry extension)
# Returns: 0 if test file, 1 otherwise
is_test_file() {
  local name="$1"
  # Currently no files use the --test flag
  [[ "$name" == "__none__" ]]
}

# Get baseline status from baselines.json.
# Arguments: $1 = basename
# Returns: status string (PASS, FAIL, FLAKY) or empty if not found
get_baseline_status() {
  local name="$1"
  if [[ ! -f "$BASELINES_FILE" ]]; then
    echo ""
    return
  fi
  jq -r --arg n "$name" '.[$n].status // ""' "$BASELINES_FILE" 2>/dev/null || echo ""
}

# Check if a file is marked as FLAKY in baselines.json.
# Arguments: $1 = basename
# Returns: 0 if flaky, 1 otherwise
is_flaky() {
  local name="$1"
  local status
  status="$(get_baseline_status "$name")"
  [[ "$status" == "FLAKY" ]]
}

# Check if a file is marked as EXP_FAIL (expected failure) in baselines.json.
# Arguments: $1 = basename
# Returns: 0 if expected failure, 1 otherwise
is_expected_failure() {
  local name="$1"
  local status
  status="$(get_baseline_status "$name")"
  [[ "$status" == "EXP_FAIL" ]]
}

# Compile a single .ry file. Captures stdout and stderr separately.
# Arguments: $1 = input file, $2 = output binary, $3 = stdout file, $4 = stderr file
# Returns: 0 on success, 1 on failure
compile_file() {
  local input="$1"
  local output="$2"
  local stdout_file="$3"
  local stderr_file="$4"
  local name
  name="$(basename "$input" .ry)"

  local compile_args=()
  if is_test_file "$name"; then
    compile_args+=("--test")
  fi

  echo "  Compiling: $input -> $output"

  local exit_code=0
  run_with_timeout "$COMPILE_TIMEOUT" "$COMPILER" ${compile_args[@]+"${compile_args[@]}"} "$input" -o "$output" \
    </dev/null 1>"$stdout_file" 2>"$stderr_file" || exit_code=$?

  return "$exit_code"
}

# Run a compiled binary. Captures stdout and stderr separately.
# Arguments: $1 = binary path, $2 = stdout file, $3 = stderr file
# Returns: exit code of the binary
run_binary() {
  local binary="$1"
  local stdout_file="$2"
  local stderr_file="$3"

  local exit_code=0
  # Redirect stdin from /dev/null: the loop reads the file list via
  # process substitution, and a binary reading stdin would otherwise
  # consume the remaining file list and abort the run.
  run_with_timeout "$RUN_TIMEOUT" "$binary" </dev/null 1>"$stdout_file" 2>"$stderr_file" || exit_code=$?

  return "$exit_code"
}

# Log a failure to the failures.log file.
# Arguments: $1 = example name, $2 = failure reason, $3 = stderr content file
log_failure_detail() {
  local name="$1"
  local reason="$2"
  local stderr_file="$3"

  {
    echo "=== ${name}.ry ==="
    echo "Exit code: $4"
    echo "Stderr: $(cat "$stderr_file" 2>/dev/null || echo "(empty)")"
    echo ""
  } >> "$FAILURES_LOG"
}

# Record a per-file result for the final report.
# Arguments: $1 = emoji, $2 = name, $3 = detail message
record_result() {
  local emoji="$1"
  local name="$2"
  local detail="$3"
  RESULTS+=("${emoji} ${name}: ${detail}")
}

# Print the final summary report.
print_report() {
  echo ""
  echo "============================================================"
  echo "  Example Test Report"
  echo "============================================================"

  if [[ ${#RESULTS[@]} -gt 0 ]]; then
    echo ""
    for result in "${RESULTS[@]}"; do
      echo "  $result"
    done
    echo ""
  fi

  echo "------------------------------------------------------------"
  echo "  Total:              $TOTAL"
  echo "  Passed:             $PASSED"
  echo "  Expected failures:  $EXP_FAILED"
  echo "  Failed:             $FAILED"
  echo "  Flaky (warning):    $FLAKY"
  echo "  Compilation errors: $COMPILATION_FAILURES"
  echo "  Skipped:            $SKIPPED"
  echo "------------------------------------------------------------"

  if [[ $FAILED -gt 0 ]]; then
    echo "  Status: FAILED"
    echo "============================================================"
    return 1
  elif [[ $EXP_FAILED -gt 0 ]]; then
    echo "  Status: PASSED WITH EXPECTED FAILURES"
    echo "============================================================"
    return 0
  else
    echo "  Status: ALL PASSED"
    echo "============================================================"
    return 0
  fi
}

# ── Main ─────────────────────────────────────────────────────────────────────

main() {
  mkdir -p "$TARGET_DIR"

  : > "$FAILURES_LOG"

  if [[ ! -x "$COMPILER" ]]; then
    echo "Error: Compiler not found at $COMPILER" >&2
    echo "Run 'cargo build --release' first." >&2
    exit 1
  fi

  echo "Ruyi Example Runner"
  echo "Mode: $MODE"
  echo "Compiler: $COMPILER"
  if [[ -n "$ONLY_PATTERN" ]]; then
    echo "Filter: '$ONLY_PATTERN'"
  fi
  echo ""

  TMPDIR_CLEANUP="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR_CLEANUP"' EXIT

  while IFS= read -r f; do
    [[ -e "$f" ]] || continue

    local basename
    basename="$(basename "$f" .ry)"

    if [[ -n "$ONLY_PATTERN" ]]; then
      # shellcheck disable=SC2254
      case "$basename" in
        $ONLY_PATTERN) ;;
        *)
          echo "  SKIP: $basename (does not match '$ONLY_PATTERN')"
          SKIPPED=$((SKIPPED + 1))
          continue
          ;;
      esac
    fi

    TOTAL=$((TOTAL + 1))

    local binary="$TARGET_DIR/$basename"
    local expected_file="$TARGET_DIR/${basename}.expected"
    local stdout_file="$TMPDIR_CLEANUP/${basename}.stdout"
    local stderr_file="$TMPDIR_CLEANUP/${basename}.stderr"

    local flaky=false
    if is_flaky "$basename"; then
      flaky=true
    fi

    local exp_fail=false
    if is_expected_failure "$basename"; then
      exp_fail=true
    fi

    case "$MODE" in
      default)
        local compile_exit=0
        compile_file "$f" "$binary" "$stdout_file" "$stderr_file" || compile_exit=$?

        if [[ $compile_exit -ne 0 ]]; then
          COMPILATION_FAILURES=$((COMPILATION_FAILURES + 1))
          log_failure_detail "$basename" "compilation error" "$stderr_file" "$compile_exit"
          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "compilation failed (FLAKY)"
            echo "  WARN: $basename — compilation failed (FLAKY, not counted as failure)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "compilation failed"
            echo "  FAIL: $basename — compilation error"
          fi
          continue
        fi

        local run_exit=0
        run_binary "$binary" "$stdout_file" "$stderr_file" || run_exit=$?

        if [[ $run_exit -ne 0 ]]; then
          log_failure_detail "$basename" "runtime error (exit $run_exit)" "$stderr_file" "$run_exit"
          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "runtime error (FLAKY)"
            echo "  WARN: $basename — runtime error (FLAKY, not counted as failure)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "runtime error"
            echo "  FAIL: $basename — runtime error"
          fi
          continue
        fi

        PASSED=$((PASSED + 1))
        record_result "✅" "$basename" "passed"
        echo "  PASS: $basename"
        ;;

      verify)
        local compile_exit=0
        compile_file "$f" "$binary" "$stdout_file" "$stderr_file" || compile_exit=$?

        if [[ $compile_exit -ne 0 ]]; then
          COMPILATION_FAILURES=$((COMPILATION_FAILURES + 1))
          log_failure_detail "$basename" "compilation error" "$stderr_file" "$compile_exit"
          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "compilation failed (FLAKY)"
            echo "  WARN: $basename — compilation failed (FLAKY)"
          elif $exp_fail; then
            EXP_FAILED=$((EXP_FAILED + 1))
            record_result "⏭️" "$basename" "compilation failed (expected)"
            echo "  INFO: $basename — compilation failed (expected)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "compilation failed"
            echo "  FAIL: $basename — compilation error"
          fi
          continue
        fi

        local run_exit=0
        run_binary "$binary" "$stdout_file" "$stderr_file" || run_exit=$?

        if [[ $run_exit -ne 0 ]]; then
          log_failure_detail "$basename" "runtime error (exit $run_exit)" "$stderr_file" "$run_exit"
          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "runtime error (FLAKY)"
            echo "  WARN: $basename — runtime error (FLAKY)"
          elif $exp_fail; then
            EXP_FAILED=$((EXP_FAILED + 1))
            record_result "⏭️" "$basename" "runtime error (expected)"
            echo "  INFO: $basename — runtime error (expected)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "runtime error"
            echo "  FAIL: $basename — runtime error"
          fi
          continue
        fi

        if [[ ! -f "$expected_file" ]]; then
          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "no expected file (FLAKY)"
            echo "  WARN: $basename — no expected file found (FLAKY)"
          elif $exp_fail; then
            EXP_FAILED=$((EXP_FAILED + 1))
            record_result "⏭️" "$basename" "no expected file (expected)"
            echo "  INFO: $basename — no expected file (expected)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "no expected file: $expected_file"
            echo "  FAIL: $basename — no expected file: $expected_file"
          fi
          continue
        fi

        if diff -q "$expected_file" "$stdout_file" > /dev/null 2>&1; then
          PASSED=$((PASSED + 1))
          record_result "✅" "$basename" "output matches"
          echo "  PASS: $basename"
        else
          echo "  DIFF: $basename — output mismatch"
          echo "  --- expected: $expected_file"
          echo "  +++ actual:   $stdout_file"
          diff "$expected_file" "$stdout_file" 2>/dev/null | head -20 | sed 's/^/    /' || true

          if $flaky; then
            FLAKY=$((FLAKY + 1))
            record_result "⚠️" "$basename" "output mismatch (FLAKY)"
            echo "  WARN: $basename — output mismatch (FLAKY, not counted as failure)"
          elif $exp_fail; then
            EXP_FAILED=$((EXP_FAILED + 1))
            record_result "⏭️" "$basename" "output mismatch (expected)"
            echo "  INFO: $basename — output mismatch (expected)"
          else
            FAILED=$((FAILED + 1))
            record_result "❌" "$basename" "output mismatch"
            echo "  FAIL: $basename — output mismatch"
          fi
        fi
        ;;

      update)
        local compile_exit=0
        compile_file "$f" "$binary" "$stdout_file" "$stderr_file" || compile_exit=$?

        if [[ $compile_exit -ne 0 ]]; then
          COMPILATION_FAILURES=$((COMPILATION_FAILURES + 1))
          log_failure_detail "$basename" "compilation error" "$stderr_file" "$compile_exit"
          record_result "❌" "$basename" "compilation failed (not updated)"
          echo "  FAIL: $basename — compilation error (not updated)"
          continue
        fi

        local run_exit=0
        run_binary "$binary" "$stdout_file" "$stderr_file" || run_exit=$?

        if [[ $run_exit -ne 0 ]]; then
          log_failure_detail "$basename" "runtime error (exit $run_exit)" "$stderr_file" "$run_exit"
          record_result "❌" "$basename" "runtime error (not updated)"
          echo "  FAIL: $basename — runtime error (not updated)"
          continue
        fi

        cp "$stdout_file" "$expected_file"
        PASSED=$((PASSED + 1))
        record_result "✅" "$basename" "expected updated"
        echo "  PASS: $basename — expected updated"
        ;;
    esac
  done < <(find "$EXAMPLES_DIR" -name '*.ry' -not -path '*/target/*' | sort)

  local report_exit=0
  print_report || report_exit=$?

  if [[ -s "$FAILURES_LOG" ]]; then
    echo ""
    echo "Detailed failure logs: $FAILURES_LOG"
  fi

  return "$report_exit"
}

main "$@"
