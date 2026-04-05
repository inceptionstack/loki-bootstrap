# Review: `v0.2.0-rc11..v0.2.0-rc19`

## CRITICAL

No critical findings in this range.

## HIGH

### 1. `enable_satellite_services` is now effectively hard-disabled for all Terraform installs

Files:
- `deploy/terraform/variables.tf:206`
- `tools/loki-installer/src/adapters/terraform.rs:444`
- `methods/terraform.yaml:12`
- `packs/openclaw/manifest.yaml:19`

What changed:
- `enable_satellite_services` now defaults to `false` in Terraform.
- The adapter can only pass `-var=enable_satellite_services=...` if `adapter_options` contains that key.
- Nothing in the Terraform method manifest or pack manifests exposes `enable_satellite_services` as a user-selectable or defaulted option.

Impact:
- Fresh Terraform installs will now skip the entire satellite-services path by default, including the Bedrock form Lambda, security enablement Lambda, and builder admin-user setup.
- The description in `variables.tf` says this should be disabled only for reinstalls or existing-VPC cases, but the current wiring makes `false` the only reachable value.

Why this is a bug:
- This is not just a safer default. It is a behavioral regression that silently removes resources from every Terraform deployment.

Recommended fix:
- Keep the Terraform variable default aligned with the intended default behavior for first installs.
- If you want reinstall-specific behavior, derive it in the installer and pass it explicitly, instead of changing the Terraform default to an unreachable value.

### 2. The environment-name persistence fix is repo-global and does not actually solve reinstall/state continuity safely

Files:
- `tools/loki-installer/src/adapters/terraform.rs:196`

What changed:
- When `resolved_stack_name` is absent, Terraform now reads `deploy/terraform/.loki-env` and reuses `ENVIRONMENT_NAME`.
- If the file does not exist, it generates a random suffix and immediately writes it back.

Impact:
- All future Terraform installs from the same checkout share one repo-local environment name, regardless of pack, profile, region, AWS account, or workspace.
- A second install from the same repo can unintentionally target the first install’s naming namespace.
- A reinstall from a fresh checkout still generates a new name, so the original “destroy old resources on reinstall” problem is not actually solved across machines or clones.

Why this is a bug:
- The fix stores identity in an unscoped local sidecar file rather than in deployment state.
- That avoids churn only in the narrow “same checkout, same repo dir” case, and creates cross-install collisions in the broader case.

Recommended fix:
- Prefer existing Terraform state first, not a random local file.
- Practical order:
  1. If the user supplied `stack_name`, use it.
  2. If Terraform state already exists for the selected workspace, read the prior `environment_name` from state before planning.
  3. Only if no state exists, generate a name once and persist it.
- If you keep a local file fallback, key it by at least workspace + pack + profile + region + AWS account, not a single repo-wide `.loki-env`.

### 3. Deploy log truncation is not safe and likely does not fix the reported garbling reliably

Files:
- `tools/loki-installer/src/tui/screens/deploy.rs:15`
- `tools/loki-installer/src/tui/runtime.rs:334`

What changed:
- The deploy screen now truncates log lines using `content_with_width`.
- Truncation is done with `line.len()` and `&line[..max_line]`.

Impact:
- `len()` is byte length, not terminal display width.
- `&line[..max_line]` can panic if `max_line` lands inside a multibyte UTF-8 character.
- This code uses the outer widget width (`body[1].width`) rather than the inner content width after borders, so even ASCII truncation is off by the block chrome.

Why this is a bug:
- The current implementation is not width-aware and is not UTF-8 safe.
- That means it can still wrap badly, and with the wrong input it can crash instead of merely truncating.

Recommended fix:
- Measure display width with `unicode-width`.
- Truncate by `chars()` or grapheme boundaries, not raw byte slicing.
- Use the inner paragraph width, not the block width, when computing available columns.

## MEDIUM

### 4. Simple mode still does not let the user see or change the preselected pack/profile before the plan is built

Files:
- `tools/loki-installer/src/tui/update.rs:280`
- `tools/loki-installer/src/tui/update.rs:307`
- `tools/loki-installer/src/tui/runtime.rs:93`

What happens:
- In simple mode, `DoctorPreflight -> LoadPacks`.
- `PacksLoaded` auto-selects `openclaw` and returns `LoadProfiles`.
- `ProfilesLoaded` auto-selects `builder` and returns `LoadMethods`.
- In `ProfileSelection`, simple mode immediately returns `BuildPlan`.
- `run_actions()` drains this queue synchronously in one pass.

Impact:
- The user does not get an interactive stop on pack or profile selection.
- They jump straight from doctor to review.
- The only way to change pack/profile is later, from review, by pressing `A` to switch modes and restart selection.

Answer to the question:
- No, not in the current simple-mode runtime flow.

Recommended fix:
- Decide which behavior you want:
  - If simple mode should still be editable, stop the queue on the pack/profile screens and wait for input.
  - If simple mode should be non-editable, make that explicit in the UX and present the chosen defaults only on review.

### 5. The new post-install summary will often show `Stack: <environment_name>` for Terraform installs

Files:
- `tools/loki-installer/src/tui/screens/post_install.rs:25`
- `methods/terraform.yaml:5`
- `tools/loki-installer/src/adapters/terraform.rs:380`

What changed:
- The post-install screen now tries `resolved_stack_name`, then `artifacts["stack_name"]`, then `request.stack_name`.
- For Terraform, `requires_stack_name` is `false`, so `resolved_stack_name` is normally `None`.
- The Terraform adapter records outputs like `instance_id`/`public_ip`, but it does not record the chosen `environment_name` as an artifact.

Impact:
- Successful Terraform installs can land on the new completion screen with a placeholder instead of the actual deployed environment name.

Recommended fix:
- Record the final `environment_name` into session artifacts during Terraform apply/output capture, and have post-install read that artifact explicitly.

## LOW

### 6. The `admin_setup_basic` count mismatch was fixed correctly

Files:
- `deploy/terraform/main.tf:638`

Assessment:
- The new `count = var.enable_satellite_services && var.profile_name == "builder" ? 1 : 0` now matches the producer resource `aws_iam_role.admin_setup_lambda[0]`.
- I did not find a remaining count/index mismatch in this specific block after the change.
