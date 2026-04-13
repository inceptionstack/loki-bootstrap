#!/usr/bin/env bash

if [[ -n "${_LOKI_WIZARD_COMMAND_SH:-}" ]]; then
  return 0
fi
_LOKI_WIZARD_COMMAND_SH=1

wizard_shell_quote() {
  printf '%q' "$1"
}

wizard_bool_to_cfn() {
  [[ "$1" == "true" ]] && printf 'true' || printf 'false'
}

build_bootstrap_command() {
  local pack profile provider provider_auth_type provider_region primary_model_override
  local provider_key provider_key_secret_id provider_base_url repo_branch gw_port
  local bedrockify_port hermes_model existing_vpc_id existing_subnet_id
  pack="$(wizard_state_get pack)"
  profile="$(wizard_state_get profile)"
  provider="$(wizard_state_get provider)"
  provider_auth_type="$(wizard_state_get providerAuthType)"
  provider_region="$(wizard_state_get providerRegion)"
  primary_model_override="$(wizard_state_get primaryModelOverride)"
  provider_key="$(wizard_state_get providerKey)"
  provider_key_secret_id="$(wizard_state_get providerKeySecretId)"
  provider_base_url="$(wizard_state_get providerBaseUrl)"
  repo_branch="$(wizard_state_get repoBranch)"
  gw_port="$(wizard_state_get gwPort)"
  bedrockify_port="$(wizard_state_get bedrockifyPort)"
  hermes_model="$(wizard_state_get hermesModel)"
  existing_vpc_id="$(wizard_state_get existingVpcId)"
  existing_subnet_id="$(wizard_state_get existingSubnetId)"
  local cmd=()
  cmd+=(bash deploy/bootstrap.sh)
  cmd+=(--pack "$(wizard_shell_quote "${pack}")")
  [[ -n "${profile}" ]] && cmd+=(--profile "$(wizard_shell_quote "${profile}")")
  [[ -n "${provider}" && "${provider}" != "own-cloud" ]] && \
    cmd+=(--provider "$(wizard_shell_quote "${provider}")")
  [[ "${provider}" == "bedrock" && -n "${provider_auth_type}" ]] && \
    cmd+=(--provider-auth-type "$(wizard_shell_quote "${provider_auth_type}")")
  [[ -n "${provider_region}" ]] && cmd+=(--region "$(wizard_shell_quote "${provider_region}")")
  [[ -n "${primary_model_override}" ]] && cmd+=(--model "$(wizard_shell_quote "${primary_model_override}")")
  [[ -n "${provider_key}" ]] && cmd+=(--provider-key "$(wizard_shell_quote "${provider_key}")")
  [[ -n "${provider_key_secret_id}" ]] && cmd+=(--provider-key-secret-id "$(wizard_shell_quote "${provider_key_secret_id}")")
  [[ -n "${provider_base_url}" ]] && cmd+=(--provider-base-url "$(wizard_shell_quote "${provider_base_url}")")
  if [[ "${provider}" == "litellm" && -n "${provider_base_url}" ]]; then
    cmd+=(--litellm-base-url "$(wizard_shell_quote "${provider_base_url}")")
  fi
  [[ -n "${repo_branch}" ]] && cmd+=(--repo-branch "$(wizard_shell_quote "${repo_branch}")")
  [[ -n "${gw_port}" ]] && cmd+=(--gw-port "$(wizard_shell_quote "${gw_port}")")
  [[ -n "${bedrockify_port}" ]] && cmd+=(--bedrockify-port "$(wizard_shell_quote "${bedrockify_port}")")
  [[ -n "${hermes_model}" ]] && cmd+=(--hermes-model "$(wizard_shell_quote "${hermes_model}")")
  [[ -n "${existing_vpc_id}" ]] && cmd+=(--existing-vpc-id "$(wizard_shell_quote "${existing_vpc_id}")")
  [[ -n "${existing_subnet_id}" ]] && cmd+=(--existing-subnet-id "$(wizard_shell_quote "${existing_subnet_id}")")
  local out=""
  local part
  for part in "${cmd[@]}"; do
    [[ -n "${out}" ]] && out+=" "
    out+="${part}"
  done
  printf '%s\n' "${out}"
}

build_cfn_params() {
  jq -n \
    --arg EnvironmentName "$(wizard_state_get environmentName)" \
    --arg PackName "$(wizard_state_get pack)" \
    --arg ProfileName "$(wizard_state_get profile)" \
    --arg InstanceType "$(wizard_state_get instanceType)" \
    --arg RootVolumeSize "$(wizard_state_get rootVolumeGb)" \
    --arg DataVolumeSize "$(wizard_state_get dataVolumeGb)" \
    --arg KeyPairName "$(wizard_state_get keyPairName)" \
    --arg BedrockRegion "$(wizard_state_get providerRegion)" \
    --arg DefaultModel "$(wizard_state_get primaryModelOverride)" \
    --arg ProviderName "$(wizard_state_get provider)" \
    --arg ProviderAuthType "$(wizard_state_get providerAuthType)" \
    --arg ProviderApiKey "$(wizard_state_get providerKey)" \
    --arg ProviderApiKeySecretArn "$(wizard_state_get providerKeySecretId)" \
    --arg LiteLLMBaseUrl "$(wizard_state_get providerBaseUrl)" \
    --arg ExistingVpcId "$(wizard_state_get existingVpcId)" \
    --arg ExistingSubnetId "$(wizard_state_get existingSubnetId)" \
    --arg SSHAllowedCidr "$(wizard_state_get sshAllowedCidr)" \
    --arg RepoBranch "$(wizard_state_get repoBranch)" \
    --arg OpenClawGatewayPort "$(wizard_state_get gwPort)" \
    --arg LokiWatermark "$(wizard_state_get lokiWatermark)" \
    --arg EnableBedrockForm "$(wizard_bool_to_cfn "$(wizard_state_get enableBedrockForm)")" \
    --arg RequestQuotaIncreases "$(wizard_bool_to_cfn "$(wizard_state_get requestQuotaIncreases)")" \
    --arg EnableSecurityHub "$(wizard_bool_to_cfn "$(wizard_state_get enableSecurityHub)")" \
    --arg EnableGuardDuty "$(wizard_bool_to_cfn "$(wizard_state_get enableGuardDuty)")" \
    --arg EnableInspector "$(wizard_bool_to_cfn "$(wizard_state_get enableInspector)")" \
    --arg EnableAccessAnalyzer "$(wizard_bool_to_cfn "$(wizard_state_get enableAccessAnalyzer)")" \
    --arg EnableConfigRecorder "$(wizard_bool_to_cfn "$(wizard_state_get enableConfigRecorder)")" \
    '{
      EnvironmentName: $EnvironmentName,
      PackName: $PackName,
      ProfileName: $ProfileName,
      InstanceType: $InstanceType,
      RootVolumeSize: $RootVolumeSize,
      DataVolumeSize: $DataVolumeSize,
      KeyPairName: $KeyPairName,
      BedrockRegion: $BedrockRegion,
      DefaultModel: $DefaultModel,
      ProviderName: $ProviderName,
      ProviderAuthType: $ProviderAuthType,
      ProviderApiKey: $ProviderApiKey,
      ProviderApiKeySecretArn: $ProviderApiKeySecretArn,
      LiteLLMBaseUrl: $LiteLLMBaseUrl,
      ExistingVpcId: $ExistingVpcId,
      ExistingSubnetId: $ExistingSubnetId,
      SSHAllowedCidr: $SSHAllowedCidr,
      RepoBranch: $RepoBranch,
      OpenClawGatewayPort: $OpenClawGatewayPort,
      LokiWatermark: $LokiWatermark,
      EnableBedrockForm: $EnableBedrockForm,
      RequestQuotaIncreases: $RequestQuotaIncreases,
      EnableSecurityHub: $EnableSecurityHub,
      EnableGuardDuty: $EnableGuardDuty,
      EnableInspector: $EnableInspector,
      EnableAccessAnalyzer: $EnableAccessAnalyzer,
      EnableConfigRecorder: $EnableConfigRecorder
    }'
}

build_terraform_vars() {
  jq -n \
    --arg environment_name "$(wizard_state_get environmentName)" \
    --arg pack_name "$(wizard_state_get pack)" \
    --arg profile_name "$(wizard_state_get profile)" \
    --arg instance_type "$(wizard_state_get instanceType)" \
    --arg root_volume_size "$(wizard_state_get rootVolumeGb)" \
    --arg data_volume_size "$(wizard_state_get dataVolumeGb)" \
    --arg key_pair_name "$(wizard_state_get keyPairName)" \
    --arg bedrock_region "$(wizard_state_get providerRegion)" \
    --arg default_model "$(wizard_state_get primaryModelOverride)" \
    --arg provider_name "$(wizard_state_get provider)" \
    --arg provider_auth_type "$(wizard_state_get providerAuthType)" \
    --arg provider_api_key "$(wizard_state_get providerKey)" \
    --arg provider_api_key_secret_arn "$(wizard_state_get providerKeySecretId)" \
    --arg provider_base_url "$(wizard_state_get providerBaseUrl)" \
    --arg litellm_base_url "$(wizard_state_get providerBaseUrl)" \
    --arg existing_vpc_id "$(wizard_state_get existingVpcId)" \
    --arg existing_subnet_id "$(wizard_state_get existingSubnetId)" \
    --arg ssh_allowed_cidr "$(wizard_state_get sshAllowedCidr)" \
    --arg repo_branch "$(wizard_state_get repoBranch)" \
    --arg openclaw_gateway_port "$(wizard_state_get gwPort)" \
    --arg loki_watermark "$(wizard_state_get lokiWatermark)" \
    --arg enable_bedrock_form "$(wizard_bool_to_cfn "$(wizard_state_get enableBedrockForm)")" \
    --arg request_quota_increases "$(wizard_bool_to_cfn "$(wizard_state_get requestQuotaIncreases)")" \
    --argjson enable_security_hub "$( [[ "$(wizard_state_get enableSecurityHub)" == "true" ]] && echo true || echo false )" \
    --argjson enable_guardduty "$( [[ "$(wizard_state_get enableGuardDuty)" == "true" ]] && echo true || echo false )" \
    --argjson enable_inspector "$( [[ "$(wizard_state_get enableInspector)" == "true" ]] && echo true || echo false )" \
    --argjson enable_access_analyzer "$( [[ "$(wizard_state_get enableAccessAnalyzer)" == "true" ]] && echo true || echo false )" \
    --argjson enable_config_recorder "$( [[ "$(wizard_state_get enableConfigRecorder)" == "true" ]] && echo true || echo false )" \
    '{
      environment_name: $environment_name,
      pack_name: $pack_name,
      profile_name: $profile_name,
      instance_type: $instance_type,
      root_volume_size: $root_volume_size,
      data_volume_size: $data_volume_size,
      key_pair_name: $key_pair_name,
      bedrock_region: $bedrock_region,
      default_model: $default_model,
      provider_name: $provider_name,
      provider_auth_type: $provider_auth_type,
      provider_api_key: $provider_api_key,
      provider_api_key_secret_arn: $provider_api_key_secret_arn,
      provider_base_url: $provider_base_url,
      litellm_base_url: $litellm_base_url,
      existing_vpc_id: $existing_vpc_id,
      existing_subnet_id: $existing_subnet_id,
      ssh_allowed_cidr: $ssh_allowed_cidr,
      repo_branch: $repo_branch,
      openclaw_gateway_port: $openclaw_gateway_port,
      loki_watermark: $loki_watermark,
      enable_bedrock_form: $enable_bedrock_form,
      request_quota_increases: $request_quota_increases,
      enable_security_hub: $enable_security_hub,
      enable_guardduty: $enable_guardduty,
      enable_inspector: $enable_inspector,
      enable_access_analyzer: $enable_access_analyzer,
      enable_config_recorder: $enable_config_recorder
    }'
}
