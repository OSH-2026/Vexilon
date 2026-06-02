#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAB4_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

MODEL_PATH="${MODEL_PATH:-$LAB4_DIR/models/your-model.gguf}"
LLAMA_CLI="${LLAMA_CLI:-$LAB4_DIR/llama.cpp/build/bin/llama-cli}"
PROMPT="${PROMPT:-请用三句话解释什么是虚拟内存，并说明页表和 TLB 的关系。}"
N_PREDICT="${N_PREDICT:-128}"
THREADS="${THREADS:-4}"
CTX_SIZE="${CTX_SIZE:-2048}"
BATCH_SIZE="${BATCH_SIZE:-256}"

mkdir -p "$LAB4_DIR/results"

LOG="$LAB4_DIR/results/single_run_log.txt"
OUT="$LAB4_DIR/results/single_inference_output.txt"
PROMPT_FILE="$LAB4_DIR/results/single_prompt.txt"

{
  echo "# Single llama-cli Run"
  echo "Start time: $(date -Iseconds 2>/dev/null || date)"
  echo "MODEL_PATH=$MODEL_PATH"
  echo "LLAMA_CLI=$LLAMA_CLI"
  echo "PROMPT=$PROMPT"
  echo "N_PREDICT=$N_PREDICT"
  echo "THREADS=$THREADS"
  echo "CTX_SIZE=$CTX_SIZE"
  echo "BATCH_SIZE=$BATCH_SIZE"
  echo
} >> "$LOG"

if [ ! -x "$LLAMA_CLI" ]; then
  echo "ERROR: llama-cli not found or not executable: $LLAMA_CLI" | tee -a "$LOG"
  exit 1
fi

if [ ! -f "$MODEL_PATH" ]; then
  echo "ERROR: model file not found: $MODEL_PATH" | tee -a "$LOG"
  echo "Please put a GGUF model under Lab4/models/ or set MODEL_PATH." | tee -a "$LOG"
  exit 1
fi

CMD=(
  "$LLAMA_CLI"
  -m "$MODEL_PATH"
  -f "$PROMPT_FILE"
  -n "$N_PREDICT"
  --threads "$THREADS"
  --ctx-size "$CTX_SIZE"
  --batch-size "$BATCH_SIZE"
  --single-turn
  --simple-io
)

printf "%s" "$PROMPT" > "$PROMPT_FILE"
echo "Command: ${CMD[*]}" | tee -a "$LOG"

START_NS="$(date +%s%N)"
"${CMD[@]}" 2>&1 | tee "$OUT"
END_NS="$(date +%s%N)"

LATENCY="$(python3 -c "print((int('$END_NS') - int('$START_NS')) / 1000000000)")"

{
  echo
  echo "End time: $(date -Iseconds 2>/dev/null || date)"
  echo "Total latency seconds: $LATENCY"
  echo "Output saved to: $OUT"
  echo
} >> "$LOG"

echo "Single inference finished. Output: $OUT"
