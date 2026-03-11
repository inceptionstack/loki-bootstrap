# BOOTSTRAP-OUTLINE-SYNC.md — Outline Wiki Integration & Workspace Sync

> **Run this once to set up automatic Outline sync.**
> If `memory/.bootstrapped-outline-sync` exists, skip.

## Overview

Loki syncs workspace documents to an Outline wiki automatically via a daily cron. This keeps the team's knowledge base current without manual effort.

Two crons run daily:
1. **`outline-sync`** — pushes workspace files to Outline
2. **`project-guidelines-audit`** — audits repos against guidelines, uploads report to Outline

## Prerequisites

- Outline instance running (see the `outline` ECS service)
- Outline API token in Secrets Manager
- `outline` skill installed (from loki-skills)

## Step 1: Configure Outline Skill

The `outline` skill (from loki-skills) handles all Outline API calls. Verify it's installed:

```bash
ls ~/.openclaw/workspace/skills/outline/
```

## Step 2: Add Outline Sync Cron

Add via OpenClaw cron:

```
/cron add "Outline workspace sync" --schedule "0 2 * * *" --message "Sync workspace documentation to Outline wiki. Upload any updated MEMORY.md, SOUL.md, AGENTS.md, HEARTBEAT.md, and any files in memory/ that changed today. Create or update documents in the FastStart Ops collection. Report what was synced."
```

This runs at 02:00 UTC daily.

## Step 3: Add Project Guidelines Audit Cron

```
/cron add "Project guidelines audit" --schedule "0 9 * * *" --message "Audit all active CodeCommit repos against our project guidelines: IaC-first, no hardcoded secrets, README present, CI/CD pipeline exists, no direct console deploys. For each repo, check compliance and list violations. Upload the full report to Outline in the Reports collection. Alert on any critical violations."
```

This runs at 09:00 UTC daily.

## Step 4: Outline Collections

Key collections to know:
- **FastStart Ops** — operational docs, runbooks, architecture notes
- **Reports** — audit reports, security digests (collection ID: `e9d42311-a3f8-4c8f-8d03-c35bfb1eecca`)

When uploading reports, always include:
- Timestamp in the document title
- Summary section at the top
- Full details below

## Step 5: Manual Sync

To trigger a sync manually, ask Loki:
> "Sync workspace to Outline"

Or use the cron tool to run it immediately:
```
/cron run outline-sync
```

## Finish

```bash
mkdir -p memory && echo "Outline sync bootstrapped $(date -u +%Y-%m-%dT%H:%M:%SZ)" > memory/.bootstrapped-outline-sync
```
