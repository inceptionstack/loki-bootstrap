#!/usr/bin/env bash

if [[ -n "${_LOKI_WIZARD_STATE_SH:-}" ]]; then
  return 0
fi
_LOKI_WIZARD_STATE_SH=1

WIZARD_STATE_FILE="/tmp/loki-wizard-state-$$.json"
trap 'rm -f "${WIZARD_STATE_FILE:-}"' EXIT

wizard_state_init() {
  jq -n '
    {
      installMode: "simple",
      environmentName: "",
      lastPackSelection: "openclaw",
      pack: "openclaw",
      profile: "builder",
      provider: "bedrock",
      providerAuthType: "iam",
      providerRegion: "us-east-1",
      providerKey: "",
      providerKeySecretId: "",
      providerBaseUrl: "",
      primaryModelOverride: "",
      fallbackModelOverride: "",
      contextWindowOverride: "",
      maxTokensOverride: "",
      instanceType: "",
      rootVolumeGb: "",
      dataVolumeGb: "",
      vpcMode: "new",
      existingVpcId: "",
      existingSubnetId: "",
      sshAccessMode: "ssm-only",
      keyPairName: "",
      sshAllowedCidr: "127.0.0.1/32",
      telegramToken: "",
      allowedChatIds: "",
      deployMethod: "cfn-cli",
      repoBranch: "main",
      gwPort: "3001",
      bedrockifyPort: "",
      hermesModel: "",
      lokiWatermark: "",
      enableBedrockForm: "false",
      requestQuotaIncreases: "false",
      enableSecurityHub: "true",
      enableGuardDuty: "true",
      enableInspector: "true",
      enableAccessAnalyzer: "true",
      enableConfigRecorder: "true",
      generatedBootstrapCommand: "",
      generatedCfnParams: "",
      generatedTerraformVars: "",
      deployResult: ""
    }
  ' > "${WIZARD_STATE_FILE}"
}

wizard_state_get() {
  local key="$1"
  jq -r --arg k "${key}" '.[$k] // ""' "${WIZARD_STATE_FILE}"
}

wizard_state_set() {
  local key="$1"
  local value="${2-}"
  local tmp_file="${WIZARD_STATE_FILE}.tmp"
  jq --arg k "${key}" --arg v "${value}" '. + {($k): $v}' "${WIZARD_STATE_FILE}" > "${tmp_file}" &&
    mv "${tmp_file}" "${WIZARD_STATE_FILE}"
}

wizard_state_set_many() {
  local patch_json='{}'
  local key value tmp_file
  while [[ $# -gt 1 ]]; do
    key="$1"
    value="$2"
    patch_json="$(jq -c --arg k "${key}" --arg v "${value}" '. + {($k): $v}' <<<"${patch_json}")"
    shift 2
  done
  tmp_file="${WIZARD_STATE_FILE}.tmp"
  jq -s '.[0] * .[1]' "${WIZARD_STATE_FILE}" <(printf '%s\n' "${patch_json}") > "${tmp_file}" &&
    mv "${tmp_file}" "${WIZARD_STATE_FILE}"
}

wizard_state_json() {
  jq -S . "${WIZARD_STATE_FILE}"
}

wizard_state_apply_json() {
  local json="$1"
  local tmp_file="${WIZARD_STATE_FILE}.tmp"
  jq -s '.[0] * .[1]' "${WIZARD_STATE_FILE}" <(printf '%s\n' "${json}") > "${tmp_file}" &&
    mv "${tmp_file}" "${WIZARD_STATE_FILE}"
}

wizard_state_export_json_file() {
  local path="$1"
  cp "${WIZARD_STATE_FILE}" "${path}"
}
