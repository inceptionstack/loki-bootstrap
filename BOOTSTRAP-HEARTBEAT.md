# BOOTSTRAP-HEARTBEAT.md — Heartbeat & Autonomous Monitoring Setup

> **Run this once to configure your autonomous monitoring loop.**
> If `memory/.bootstrapped-heartbeat` exists, skip.

## Overview

OpenClaw fires a heartbeat event periodically when idle. Loki uses this to autonomously monitor pipelines, tasks, and security — so the operator doesn't have to ask.

The heartbeat response must be either `HEARTBEAT_OK` (nothing to do) or an alert/action message. Never a blank response.

## Step 1: Create HEARTBEAT.md

Create `~/.openclaw/workspace/HEARTBEAT.md`:

```markdown
# Heartbeat Checklist

## 1. Task Board (every heartbeat when idle)
Check `faststart-tasks` DynamoDB for work. Priority: in-progress → verify done → top backlog.

**Lifecycle:** Move to in-progress → do work → wait for pipeline green → test DEPLOYED version → move to done → notify operator → ONE task at a time.

Skip if operator is actively chatting.

## 2. Pipeline Health (every heartbeat)
Check all active CodePipelines for failures. On failure: check CodeBuild logs, fix, push, wait for green.

## 3. Security Monitoring (every heartbeat)
Check GuardDuty, Security Hub, Access Analyzer, Inspector for HIGH/CRITICAL findings.
Alert operator immediately on new critical findings. Log low/medium silently.
```

Customize the pipeline names and DynamoDB table for your environment.

## Step 2: Task Board DynamoDB Table

The task board uses a DynamoDB table (default: `faststart-tasks`). Create it if it doesn't exist:

```bash
aws dynamodb create-table \
  --table-name faststart-tasks \
  --attribute-definitions AttributeName=id,AttributeType=S \
  --key-schema AttributeName=id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST \
  --region us-east-1
```

**Task schema:**
```json
{
  "id": "task-001",
  "title": "Add error handling to API",
  "status": "backlog",
  "priority": "high",
  "notes": "See issue #42"
}
```

**Status lifecycle:** `backlog` → `in-progress` → `done`

**Rules:**
- Only ONE task in-progress at a time
- Always verify work is deployed and tested before moving to `done`
- Notify operator when a task is completed
- Skip task board if operator is actively chatting (prioritize conversation)

## Step 3: Pipeline Notifications (Optional)

To get notified when pipelines fail, deploy the pipeline notifier Lambda:

```bash
# See faststart-ops repo for the CloudFormation template
# Sends failures to Telegram + OpenClaw main session via EventBridge
```

The pattern: EventBridge rule (CodePipeline state change) → Lambda → Telegram + `openclaw system event`.

## Step 4: Security Monitoring Crons

Add a daily security digest cron via OpenClaw:

```
/cron add "Daily security check" --schedule "0 9 * * *" --message "Run a security check: GuardDuty, Security Hub, Access Analyzer. Summarize any new HIGH/CRITICAL findings since yesterday and alert if anything new."
```

## Heartbeat Response Rules

- `HEARTBEAT_OK` — nothing needs attention, operator not waiting
- Any other text — you found something, are taking action, or alerting
- **Never** include `HEARTBEAT_OK` in a real response (partial match disables it)
- Skip all checks if operator is actively in a conversation

## Finish

```bash
mkdir -p memory && echo "Heartbeat bootstrapped $(date -u +%Y-%m-%dT%H:%M:%SZ)" > memory/.bootstrapped-heartbeat
```
