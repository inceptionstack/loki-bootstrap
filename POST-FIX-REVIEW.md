# Post-Fix Review — v2-cli Commit

**Reviewer:** Loki@FastStart (subagent)
**Date:** 2026-04-05
**Scope:** Latest commit only — verifying 5 claimed fixes from FINAL-REVIEW.md

---

## Fix 1: DISPATCHER EXIT CODE (install.sh) — ✅ Correct

**Before:** `if ! download_and_verify_v2 ...; then case "$?"` — the `!` negation causes `$?` to be 0 in the `then` block (the negation succeeded), so the case statement never matched specific exit codes.

**After:**
```bash
if download_and_verify_v2 ...; then
  download_rc=0
else
  download_rc=$?
fi
```

This is correct. In bash, `$?` inside `else` preserves the actual exit code of the failed command. Verified empirically. The `set -euo pipefail` is properly handled since the `if` construct suppresses `set -e` for the tested command.

**No remaining pitfalls.**

---

## Fix 2: BEDROCKIFY MANIFEST — ✅ Correct

**Change:** Removed `cfn` from `supported_methods`, changed `default_method` from `cfn` to `terraform`.

**Assessment:** The original review noted bedrockify was NOT in the CFN template's `PackName` AllowedValues, so CFN deployment would reject it. Removing CFN as a supported method and defaulting to terraform-only is the right call — aligns manifest with reality.

---

## Fix 3: PACK OPTIONS PASSTHROUGH — ✅ Correct

**Before:** Only `model` and `port` were hardcoded. All other pack-specific options silently dropped.

**After:** Generic mapping functions `adapter_option_to_cfn_parameter()` and `adapter_option_to_tf_var()` iterate all adapter_options.

**CFN mappings (all correct):**
- model → DefaultModel, port → OpenClawGatewayPort (existing)
- bedrockify_port → BedrockifyPort (Number, 1024-65535, default 8090) ✅
- embed_model → EmbedModel (String, has default) ✅
- hermes_model → HermesModel (String, has default) ✅
- haiku_model → HaikuModel (String, has default) ✅
- sandbox_name → SandboxName (String, has default) ✅
- telegram_token → TelegramToken (NoEcho: true, default '') ✅
- allowed_chat_ids → AllowedChatIds (String, default '') ✅

**TF mappings mirror CFN.** `telegram_token` marked `sensitive = true`. Port validation in both CFN and TF.

**CFN safety for existing stacks:** All new parameters have defaults. update-stack won't break. ✅
**TF backward compatibility:** All new variables have defaults. ✅

**Bootstrap scripts:** Both `bootstrap.sh` and `test-bootstrap.sh` updated. `claude-code/install.sh` key fixed from `haiku-model` to `haiku_model` to match JSON format. ✅

**One note:** Future pack options not added to the mapping will be silently dropped. Acceptable (explicit > blind passthrough) but should be documented.

---

## Fix 4: SESSION PERMISSIONS — ✅ Correct

**Before:** `fs::write()` with default umask (0644). Sessions potentially world-readable.

**After:**
- `open_private_file()` uses `OpenOptions::mode(0o600)` — mode set atomically at `open(2)` syscall. **No race condition** — file is never world-readable, even momentarily.
- Session directory set to 0700.
- `latest.json` also written via `write_private_file()`.
- Post-rename `set_file_permissions()` is belt-and-suspenders (rename preserves perms).

**Test:** Verifies `mode & 0o777 == 0o600` for both session file and `latest.json`. ✅

---

## Fix 5: ENVIRONMENTNAME DEDUP — ✅ Correct

**Before:** Potential for EnvironmentName appearing twice in CFN command.

**After:** `EnvironmentName` pushed once (line 356). `adapter_option_to_cfn_parameter()` does NOT map any key to EnvironmentName, so the adapter_options loop cannot produce a duplicate. `build_apply_stack_command()` hardcodes PackName/ProfileName separately, then iterates parameter_overrides containing EnvironmentName exactly once.

**Test:** Explicitly asserts EnvironmentName appears exactly once:
```rust
.filter(|arg| *arg == "ParameterKey=EnvironmentName,ParameterValue=loki-openclaw")
.count(), 1
```
✅

---

## NEW BUGS INTRODUCED?

**None found.** The refactoring is mechanical and clean. Minor rustfmt reformattings add diff noise but are harmless.

---

## TEST QUALITY

| Test | Verdict |
|---|---|
| CFN parameter mapping unit test | ✅ Covers all 7 new options + non-parameters return None |
| CFN EnvironmentName dedup assertion | ✅ Directly verifies the fix |
| TF var mapping unit test | ✅ Mirrors CFN coverage |
| TF plan command test (updated) | ✅ Passes pack-specific vars, asserts presence |
| Session permissions test | ✅ Checks actual mode bits on unix |

**Not superficial** — tests directly verify the claimed fixes.

---

## SUMMARY

| # | Fix | Rating |
|---|---|---|
| 1 | Dispatcher exit code | ✅ Correct |
| 2 | Bedrockify manifest | ✅ Correct |
| 3 | Pack options passthrough | ✅ Correct |
| 4 | Session permissions | ✅ Correct |
| 5 | EnvironmentName dedup | ✅ Correct |

**Overall: All 5 fixes correctly implemented. No new bugs introduced. Tests are meaningful. CFN and TF changes backward-compatible. Ready to merge.**
