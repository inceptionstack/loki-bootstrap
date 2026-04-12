#!/usr/bin/env bash

if [[ -n "${_LOKI_WIZARD_DATA_SH:-}" ]]; then
  return 0
fi
_LOKI_WIZARD_DATA_SH=1

WIZARD_DATA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WIZARD_PACKS_JSON=""
WIZARD_PROVIDERS_JSON=""
WIZARD_MATRIX_JSON=""
WIZARD_BEDROCK_REGIONS_JSON=""

wizard_data_require() {
  command -v python3 >/dev/null 2>&1 || {
    echo "python3 is required" >&2
    return 1
  }
  command -v jq >/dev/null 2>&1 || {
    echo "jq is required" >&2
    return 1
  }
}

wizard_data_load() {
  wizard_data_require || return 1

  local payload
  payload="$(
    WIZARD_REPO_ROOT="${WIZARD_DATA_DIR}" python3 - <<'PY'
import glob
import json
import os
import re
from pathlib import Path

import yaml

root = Path(os.environ["WIZARD_REPO_ROOT"])
registry = yaml.safe_load((root / "packs" / "registry.yaml").read_text()) or {}

providers = {}
for path in sorted(glob.glob(str(root / "providers" / "*" / "manifest.yaml"))):
    manifest = yaml.safe_load(Path(path).read_text()) or {}
    name = manifest.get("name")
    if not name:
        continue
    providers[name] = manifest

template_text = (root / "deploy" / "cloudformation" / "template.yaml").read_text()
regions = []
match = re.search(r"(?ms)^  BedrockRegion:\n.*?^    AllowedValues:\n(?P<body>(?:      - .*\n)+)", template_text)
if match:
    for line in match.group("body").splitlines():
        line = line.strip()
        if line.startswith("- "):
            regions.append(line[2:])

result = {
    "defaults": registry.get("defaults") or {},
    "packs": registry.get("packs") or {},
    "providers": providers,
    "bedrockRegions": regions,
}
print(json.dumps(result))
PY
  )" || return 1

  WIZARD_PACKS_JSON="$(jq -c '.packs' <<<"${payload}")"
  WIZARD_PROVIDERS_JSON="$(jq -c '.providers' <<<"${payload}")"
  WIZARD_BEDROCK_REGIONS_JSON="$(jq -c '.bedrockRegions' <<<"${payload}")"
  WIZARD_DEFAULTS_JSON="$(jq -c '.defaults' <<<"${payload}")"
  WIZARD_MATRIX_JSON="$(wizard_data_build_matrix)"
}

wizard_data_build_matrix() {
  WIZARD_PACKS_JSON="${WIZARD_PACKS_JSON}" WIZARD_PROVIDERS_JSON="${WIZARD_PROVIDERS_JSON}" python3 - <<'PY'
import json
import os

packs = json.loads(os.environ["WIZARD_PACKS_JSON"])
providers = json.loads(os.environ["WIZARD_PROVIDERS_JSON"])
matrix = {}

for pack, pack_meta in packs.items():
    matrix[pack] = {}
    pack_supported = pack_meta.get("supported_providers") or []
    for provider, provider_meta in providers.items():
        provider_supported = ((provider_meta.get("compatibility") or {}).get("packs"))
        if provider not in pack_supported:
            matrix[pack][provider] = {"supported": False, "reason": "pack does not support provider"}
        elif provider_supported is not None and pack not in provider_supported:
            matrix[pack][provider] = {"supported": False, "reason": "provider manifest excludes pack"}
        else:
            matrix[pack][provider] = {"supported": True, "reason": ""}

print(json.dumps(matrix, separators=(",", ":")))
PY
}

wizard_pack_ids() {
  jq -r 'to_entries[] | select(.value.type != "base") | .key' <<<"${WIZARD_PACKS_JSON}"
}

wizard_provider_ids() {
  jq -r 'keys[]' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_pack_json() {
  local pack="$1"
  jq -c --arg p "${pack}" '.[$p] // {}' <<<"${WIZARD_PACKS_JSON}"
}

wizard_provider_json() {
  local provider="$1"
  jq -c --arg p "${provider}" '.[$p] // {}' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_matrix_json() {
  printf '%s\n' "${WIZARD_MATRIX_JSON}"
}

wizard_pack_provider_status_json() {
  local pack="$1"
  local provider="$2"
  jq -c --arg pack "${pack}" --arg provider "${provider}" \
    '.[$pack][$provider] // {"supported": false, "reason": "unknown compatibility"}' \
    <<<"${WIZARD_MATRIX_JSON}"
}

wizard_pack_provider_supported() {
  local pack="$1"
  local provider="$2"
  jq -e '.supported == true' <<<"$(wizard_pack_provider_status_json "${pack}" "${provider}")" >/dev/null
}

wizard_provider_default() {
  local provider="$1"
  local field="$2"
  jq -r --arg p "${provider}" --arg f "${field}" '.[$p].defaults[$f] // ""' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_model_ids() {
  local provider="$1"
  jq -r --arg p "${provider}" '.[$p].models[]?.id' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_model_json() {
  local provider="$1"
  local model_id="$2"
  jq -c --arg p "${provider}" --arg m "${model_id}" \
    '(.[$p].models // [] | map(select(.id == $m)) | .[0]) // {}' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_modes() {
  local provider="$1"
  jq -r --arg p "${provider}" '.[$p].auth.modes[]?' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_default_mode() {
  local provider="$1"
  jq -r --arg p "${provider}" '.[$p].auth.defaultMode // ""' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_region_required() {
  local provider="$1"
  jq -r --arg p "${provider}" '.[$p].connection.regionRequired // false' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_provider_display_name() {
  local provider="$1"
  jq -r --arg p "${provider}" '.[$p].displayName // $p' <<<"${WIZARD_PROVIDERS_JSON}"
}

wizard_pack_display_name() {
  local pack="$1"
  case "${pack}" in
    openclaw) printf 'OpenClaw' ;;
    claude-code) printf 'Claude Code' ;;
    hermes) printf 'Hermes' ;;
    pi) printf 'Pi' ;;
    ironclaw) printf 'IronClaw' ;;
    nemoclaw) printf 'NemoClaw' ;;
    kiro-cli) printf 'Kiro CLI' ;;
    *) printf '%s' "${pack}" ;;
  esac
}

wizard_pack_default_field() {
  local pack="$1"
  local field="$2"
  jq -r --arg p "${pack}" --arg f "${field}" \
    '.[$p][$f] // ""' <<<"${WIZARD_PACKS_JSON}"
}

wizard_global_default_field() {
  local field="$1"
  jq -r --arg f "${field}" '.[$f] // ""' <<<"${WIZARD_DEFAULTS_JSON}"
}

wizard_bedrock_regions() {
  jq -r '.[]' <<<"${WIZARD_BEDROCK_REGIONS_JSON}"
}
