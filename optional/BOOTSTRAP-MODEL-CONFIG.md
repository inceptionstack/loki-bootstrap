# BOOTSTRAP-MODEL-CONFIG.md — Configure AI Models

> **Run this FIRST — before all other bootstraps.**
> Sets the right models for each context so you get quality where it matters and cost savings everywhere else.
> If `memory/.bootstrapped-model-config` exists, skip.

## Model Strategy

- **Opus 4.6** — default for all interactive sessions (direct chat, sub-agents, coding tasks)
- **Sonnet 4.6** — for heartbeats and cron jobs (automated, background work that doesn't need heavy reasoning)

This gives you full Opus quality when talking to your human, while keeping automated/scheduled work cost-efficient.

## Step 1: Configure Default Model + Heartbeat

Run this OpenClaw config patch:

```bash
openclaw config patch <<'EOF'
{
  "agents": {
    "defaults": {
      "model": {
        "primary": "amazon-bedrock/global.anthropic.claude-opus-4-6-v1"
      },
      "heartbeat": {
        "model": "amazon-bedrock/global.anthropic.claude-sonnet-4-6"
      }
    }
  }
}
EOF
```

OpenClaw restarts automatically.

## Step 2: Configure Cron Jobs

All cron jobs with `payload.kind: "agentTurn"` should set their model to Sonnet 4.6.

When creating new cron jobs, always include the model field:

```json
{
  "payload": {
    "kind": "agentTurn",
    "message": "...",
    "model": "amazon-bedrock/global.anthropic.claude-sonnet-4-6"
  }
}
```

To update existing cron jobs that don't have a model set:

```bash
# List all cron jobs
openclaw cron list

# Update each job to use Sonnet
openclaw cron update <jobId> --model "amazon-bedrock/global.anthropic.claude-sonnet-4-6"
```

## Step 3: Verify

```bash
openclaw config get agents.defaults.model
```

Expected output:
```json
{
  "primary": "amazon-bedrock/global.anthropic.claude-opus-4-6-v1"
}
```

```bash
openclaw config get agents.defaults.heartbeat.model
```

Expected output:
```
amazon-bedrock/global.anthropic.claude-sonnet-4-6
```

## Why `global.` prefix?

The `global.` inference profile routes across all AWS regions automatically — no need to pick `us.` or `eu.`. Better availability, same price.

**Critical:** Use exact model IDs — Sonnet has no `-v1` suffix, Opus does:
- ✅ `global.anthropic.claude-sonnet-4-6` (no `-v1`)
- ✅ `global.anthropic.claude-opus-4-6-v1` (has `-v1`)

Mixing these up causes invocation errors.

## Finish

```bash
mkdir -p memory && echo "Model config bootstrapped $(date -u +%Y-%m-%dT%H:%M:%SZ)" > memory/.bootstrapped-model-config
```
