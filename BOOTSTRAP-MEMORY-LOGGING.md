# BOOTSTRAP-MEMORY-LOGGING.md — Daily Memory Logging & Long-Term Recall

> **Run this once to establish the memory logging pattern.**
> If `memory/.bootstrapped-memory-logging` exists, skip.

## Overview

Loki maintains two tiers of memory:

- **Daily logs** (`memory/YYYY-MM-DD.md`) — raw notes written throughout each session
- **Long-term memory** (`MEMORY.md`) — curated, distilled facts reviewed every session

This pattern survives restarts, model changes, and long gaps between sessions.

## Step 1: Create the Memory Directory

```bash
mkdir -p ~/.openclaw/workspace/memory
```

## Step 2: Daily Log Pattern

At the **start** of each session, read today's and yesterday's daily log:
```
memory/YYYY-MM-DD.md  (today)
memory/YYYY-MM-DD.md  (yesterday)
```

Throughout the session, **write things down as they happen**:
- Decisions made and why
- Resources created (ARNs, URLs, IDs)
- Bugs fixed and root causes
- Operator preferences and directives
- Anything that would be useful to know next session

Example daily log entry:
```markdown
# 2026-03-11 — Daily Log

## CloudFront Auth Fix
- Lambda@Edge v6 deployed — fixes /signedout rewrite
- KEY LESSON: Lambda@Edge has no env vars — all config must be hardcoded

## New Resource
- DynamoDB table `faststart-tasks` created (PAY_PER_REQUEST, us-east-1)

## Operator Directives
- Roy: "No manual deploys ever. Everything through CodePipeline."
```

## Step 3: MEMORY.md — Long-Term Distillation

`MEMORY.md` is only for the main session (not group chats — security risk).

Periodically review daily logs and distill important facts into MEMORY.md:

```markdown
# MEMORY.md — Long-Term Memory

## Active Rules (Operator Directives)
- NO MANUAL DEPLOYS — all code through CI/CD pipelines
- ALL secrets in Secrets Manager — never in env files or .bashrc
- Security spend approved — enable monitoring freely, report costs after

## Hard Lessons (Don't Repeat These)
- [specific gotchas from daily logs]

## Active Projects
- [project names, URLs, repos]

## AWS Environment
- Management Account: XXXXXXXXXXXX | Region: us-east-1
```

**Sections to maintain in MEMORY.md:**
- **Active Rules** — operator directives that apply to all work
- **Hard Lessons** — specific bugs, limits, and gotchas hit in practice
- **AWS Environment** — account IDs, key resources
- **Active Projects** — quick reference to what's running

## Step 4: Read Order Each Session

Add to `AGENTS.md` (every session instructions):

```markdown
## Every Session
1. Read SOUL.md — who you are
2. Read USER.md — who you're helping
3. Read memory/YYYY-MM-DD.md (today + yesterday) for recent context
4. Main session only: Also read MEMORY.md
```

## Step 5: Memory Search Integration

With memory search enabled (see BOOTSTRAP-MEMORY-SEARCH.md), OpenClaw indexes all `memory/*.md` files and `MEMORY.md` for semantic search. This means:

- `memory_search` finds relevant context even from older daily logs
- Explicit recall of specific events without reading every file
- Hybrid search (70% vector + 30% text) for best results

## Rules

- **"Mental notes" don't survive restarts. Files do.** Write everything down.
- Don't put sensitive data (tokens, passwords) in memory files — use Secrets Manager
- Periodically review and prune old daily logs (keep last 30 days)
- Distill weekly: pull key facts from the week's logs into MEMORY.md

## Finish

```bash
mkdir -p memory
echo "Memory logging bootstrapped $(date -u +%Y-%m-%dT%H:%M:%SZ)" > memory/.bootstrapped-memory-logging
```
