# Bash 3.2 Compatibility Refactor — Task List

**Goal:** Make wizard.sh work on macOS stock bash 3.2+ (no declare -gA, no mapfile, no associative arrays).

**Strategy:** Replace `declare -gA WIZARD_STATE` associative array with a JSON tmpfile managed via jq (already a dependency).

## Tasks

### Phase 1: wizard-state.sh rewrite
- [x] Replace `declare -gA WIZARD_STATE=()` with `WIZARD_STATE_FILE="/tmp/loki-wizard-state-$$.json"`
- [x] `wizard_state_init()` → write initial JSON object to `$WIZARD_STATE_FILE`
- [x] `wizard_state_get()` → `jq -r --arg k "$1" '.[$k] // ""' "$WIZARD_STATE_FILE"`
- [x] `wizard_state_set()` → `jq --arg k "$1" --arg v "$2" '. + {($k): $v}' "$WIZARD_STATE_FILE" > tmp && mv tmp "$WIZARD_STATE_FILE"`
- [x] `wizard_state_set_many()` → loop pairs, build jq args, single write
- [x] `wizard_state_json()` → `jq -S . "$WIZARD_STATE_FILE"`
- [x] `wizard_state_apply_json()` → `jq -s '.[0] * .[1]'` merge
- [x] `wizard_state_export_json_file()` → `cp "$WIZARD_STATE_FILE" "$1"`
- [x] Remove `mapfile -t keys` in `wizard_state_apply_json` — use while-read loop
- [x] Add cleanup trap: `trap 'rm -f "$WIZARD_STATE_FILE"' EXIT`

### Phase 2: wizard.sh — replace all WIZARD_STATE[key] reads
- [x] Replace all `"${WIZARD_STATE[key]}"` reads with `"$(wizard_state_get key)"`
- [x] Replace all `${WIZARD_STATE[key]}` (unquoted) with `$(wizard_state_get key)`
- [x] ~262 replacements in wizard.sh

### Phase 3: wizard.sh — replace all WIZARD_STATE[key]=val writes
- [x] Replace all `WIZARD_STATE[key]="val"` with `wizard_state_set key "val"`
- [x] Replace all `WIZARD_STATE[key]="${var}"` with `wizard_state_set key "${var}"`
- [x] Handle multi-set blocks efficiently (batch into wizard_state_set_many where adjacent)

### Phase 4: wizard-command.sh — fix state_ref usage
- [x] Check if wizard-command.sh uses `declare -n state_ref` (nameref) — if so, refactor
- [x] All `${state_ref[key]}` reads → use wizard_state_get or passed JSON

### Phase 5: Replace mapfile in wizard.sh
- [x] Line 548: `mapfile -t supported < <(...)` → `while IFS= read -r line; do supported+=("$line"); done < <(...)`
- [x] Line 1278: `mapfile -d '' -t cfn_params < <(...)` → `while IFS= read -r -d '' item; do cfn_params+=("$item"); done < <(...)`
- [x] Line 1279: `mapfile -d '' -t template_arg < <(...)` → same pattern

### Phase 6: Verify no other bash 4+ features
- [x] No `${!array[@]}` on associative arrays (should be gone after phase 2-3)
- [x] No `declare -g` anywhere
- [x] No `declare -A` anywhere
- [x] No `|&` (pipefail redirect) — check
- [x] No `readarray` synonyms
- [x] `bash -n` syntax check passes
- [ ] Test with bash 3.2 if available (or Docker)

### Phase 7: Final validation
- [x] All 19 `--scenario N --dry-run -y` pass
- [x] `bash -n deploy/wizard.sh` passes
- [x] `bash -n deploy/lib/wizard-state.sh` passes
- [x] `bash -n deploy/lib/wizard-command.sh` passes
- [x] `bash -n deploy/lib/wizard-validate.sh` passes
- [x] No `declare -g`, `declare -A`, `mapfile`, `readarray` in any wizard file
- [x] Commit with clear message
