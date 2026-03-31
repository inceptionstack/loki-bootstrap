#!/usr/bin/env bash
# packs/openclaw/install.sh — Install OpenClaw and start the gateway service
#
# Usage:
#   ./install.sh [OPTIONS]
#
# Assumes:
#   - node/npm available (via mise or system)
#   - python3 available
#   - systemd available (user session)
#   - ~/.openclaw/ directory exists (or will be created)
#   - loginctl linger already enabled for running user (by dispatcher)
#
# Idempotent: safe to re-run.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=../common.sh
source "${SCRIPT_DIR}/../common.sh"

# ── Defaults ──────────────────────────────────────────────────────────────────
PACK_ARG_REGION="us-east-1"
PACK_ARG_MODEL="us.anthropic.claude-opus-4-6-v1"
PACK_ARG_PORT="3001"
PACK_ARG_TOKEN=""
PACK_ARG_MODEL_MODE="bedrock"
PACK_ARG_LITELLM_URL=""
PACK_ARG_LITELLM_KEY=""
PACK_ARG_LITELLM_MODEL="claude-opus-4-6"
PACK_ARG_PROVIDER_KEY=""

# ── Help ──────────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install OpenClaw and configure the gateway service.

Options:
  --region         AWS region for Bedrock         (default: us-east-1)
  --model          Default Bedrock model ID        (default: us.anthropic.claude-opus-4-6-v1)
  --port           Gateway port                    (default: 3001)
  --token          Gateway auth token              (default: auto-generated)
  --model-mode     bedrock | litellm | provider-key (default: bedrock)
  --litellm-url    LiteLLM base URL (litellm mode)
  --litellm-key    LiteLLM API key  (litellm mode)
  --litellm-model  LiteLLM model ID (litellm mode, default: claude-opus-4-6)
  --provider-key   Anthropic API key (provider-key mode)
  --help           Show this help message

Examples:
  ./install.sh --region us-east-1 --model us.anthropic.claude-opus-4-6-v1 --port 3001
  ./install.sh --model-mode litellm --litellm-url http://proxy:4000 --litellm-key sk-xxx
EOF
}

# ── Arg parsing ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)        usage; exit 0 ;;
    --region)         PACK_ARG_REGION="$2";         shift 2 ;;
    --model)          PACK_ARG_MODEL="$2";           shift 2 ;;
    --port|--gw-port) PACK_ARG_PORT="$2";            shift 2 ;;
    --token)          PACK_ARG_TOKEN="$2";           shift 2 ;;
    --model-mode)     PACK_ARG_MODEL_MODE="$2";      shift 2 ;;
    --litellm-url|--litellm-base-url)    PACK_ARG_LITELLM_URL="$2";     shift 2 ;;
    --litellm-key|--litellm-api-key)     PACK_ARG_LITELLM_KEY="$2";     shift 2 ;;
    --litellm-model)  PACK_ARG_LITELLM_MODEL="$2";   shift 2 ;;
    --provider-key|--provider-api-key)   PACK_ARG_PROVIDER_KEY="$2";    shift 2 ;;
    *) warn "Unknown argument: $1"; shift ;;
  esac
done

REGION="${PACK_ARG_REGION}"
MODEL="${PACK_ARG_MODEL}"
GW_PORT="${PACK_ARG_PORT}"
GW_TOKEN="${PACK_ARG_TOKEN}"
MODEL_MODE="${PACK_ARG_MODEL_MODE}"
LITELLM_URL="${PACK_ARG_LITELLM_URL}"
LITELLM_KEY="${PACK_ARG_LITELLM_KEY}"
LITELLM_MODEL="${PACK_ARG_LITELLM_MODEL}"
PROVIDER_KEY="${PACK_ARG_PROVIDER_KEY}"

pack_banner "openclaw"
log "region=${REGION} model=${MODEL} port=${GW_PORT} mode=${MODEL_MODE}"

# ── Prerequisites ─────────────────────────────────────────────────────────────
step "Checking prerequisites"
require_cmd node npm python3 openssl

NODE_VERSION="$(node --version 2>/dev/null || echo unknown)"
ok "node found: ${NODE_VERSION}"

# ── Install OpenClaw ──────────────────────────────────────────────────────────
step "Installing OpenClaw"

npm install -g openclaw

# Reshim if mise is available
if command -v mise &>/dev/null; then
  mise reshim 2>/dev/null || true
fi

# Ensure openclaw is on PATH for current session
NODE_PREFIX="$(npm prefix -g)"
export PATH="${NODE_PREFIX}/bin:$PATH"

if ! command -v openclaw &>/dev/null; then
  fail "openclaw command not found after npm install"
fi

OC_VERSION="$(openclaw --version 2>/dev/null || echo unknown)"
ok "OpenClaw installed: ${OC_VERSION}"

# ── Workspace and state dir ───────────────────────────────────────────────────
step "Workspace setup"
mkdir -p "${HOME}/.openclaw/workspace"
chmod 700 "${HOME}/.openclaw"
chmod 700 "${HOME}/.openclaw/workspace"
ok "Workspace ready: ${HOME}/.openclaw/workspace"

# ── Generate token if not provided ────────────────────────────────────────────
if [[ -z "${GW_TOKEN}" ]]; then
  GW_TOKEN="$(openssl rand -hex 24)"
  log "Generated gateway token"
fi

# ── Generate OpenClaw config ──────────────────────────────────────────────────
step "Generating OpenClaw config"

CONFIG_GEN="${SCRIPT_DIR}/resources/config-gen.py"
if [[ ! -f "${CONFIG_GEN}" ]]; then
  fail "config-gen.py not found at ${CONFIG_GEN}"
fi

python3 "${CONFIG_GEN}" \
  "${REGION}"        \
  "${MODEL}"         \
  "${GW_PORT}"       \
  "${GW_TOKEN}"      \
  "${MODEL_MODE}"    \
  "${LITELLM_URL}"   \
  "${LITELLM_KEY}"   \
  "${LITELLM_MODEL}" \
  "${PROVIDER_KEY}"

chmod 600 "${HOME}/.openclaw/openclaw.json"
ok "Config written and secured (mode=${MODEL_MODE})"

# ── Install systemd user service ──────────────────────────────────────────────
step "Installing systemd user service"

NODE_BIN="$(command -v node)"
OC_MAIN="${NODE_PREFIX}/lib/node_modules/openclaw/dist/index.js"

mkdir -p "${HOME}/.config/systemd/user"

# Expand template (resources/openclaw-gateway.service.tpl)
SERVICE_TPL="${SCRIPT_DIR}/resources/openclaw-gateway.service.tpl"
if [[ ! -f "${SERVICE_TPL}" ]]; then
  fail "Service template not found at ${SERVICE_TPL}"
fi

export NODE_BIN OC_MAIN GW_PORT GW_TOKEN NODE_PREFIX OC_VERSION
export USER_HOME="${HOME}"
envsubst < "${SERVICE_TPL}" > "${HOME}/.config/systemd/user/openclaw-gateway.service"
ok "Service unit written"

# ── Enable and start service ──────────────────────────────────────────────────
step "Starting gateway service"

# Enable linger (may already be done by dispatcher, but safe to repeat)
loginctl enable-linger "$(id -un)" 2>/dev/null || true

XDG_RUNTIME_DIR="/run/user/$(id -u)"
export XDG_RUNTIME_DIR
systemctl --user daemon-reload
systemctl --user enable openclaw-gateway.service

# Stop first if already running (idempotent restart)
systemctl --user stop openclaw-gateway.service 2>/dev/null || true
systemctl --user start openclaw-gateway.service

# Wait for service to settle
sleep 3

if systemctl --user is-active openclaw-gateway.service &>/dev/null; then
  ok "Gateway service is running"
else
  warn "Gateway service may not be active yet — check: systemctl --user status openclaw-gateway.service"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
write_done_marker "openclaw"
printf "\n[PACK:openclaw] INSTALLED — gateway on :%s (systemd: openclaw-gateway)\n" "${GW_PORT}"
