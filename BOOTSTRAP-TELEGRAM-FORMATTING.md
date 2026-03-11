# BOOTSTRAP-TELEGRAM-FORMATTING.md — Telegram Messaging Rules

> **Read this once — these rules apply to every message sent via Telegram.**
> No marker file needed — add these rules permanently to SOUL.md or AGENTS.md.

## The Rules

Telegram renders markdown differently from most surfaces. These rules prevent broken formatting.

### Never Use

- **Markdown tables** — Telegram does not render them. They appear as raw pipe characters.
- **Markdown headers** (`# H1`, `## H2`) — these don't render as headers in Telegram.
- **Bare absolute paths** in media tags (`MEDIA:/home/...`, `MEDIA:~/.../`) — blocked for security.

### Instead Use

| Don't | Do instead |
|-------|-----------|
| Markdown table | Bullet list with **bold** labels |
| `# Header` | **BOLD TEXT** or ALL CAPS for emphasis |
| Long formatted lists | Short bullet points |

### Example

❌ Wrong:
```
| Model | Dims | Speed |
|-------|------|-------|
| Titan | 1024 | Fast  |
| Cohere| 1536 | Med   |
```

✅ Right:
```
**Titan Embed V2** — 1024 dims, fastest
**Cohere Embed v4** — 1536 dims, best quality
```

### Links

Wrap multiple links in `<>` to suppress embeds:
```
<https://github.com/inceptionstack/embedrock>
<https://github.com/inceptionstack/loki-bootstrap>
```

### Inline Buttons

Telegram supports inline buttons for yes/no confirmations and quick actions:
```json
buttons: [[{"text": "Yes, deploy", "callback_data": "deploy_yes", "style": "success"},
           {"text": "Cancel", "callback_data": "deploy_no", "style": "danger"}]]
```

Use these for destructive operations (deletes, deploys) to get explicit confirmation.

## Add to SOUL.md or AGENTS.md

Add this line to ensure it's always remembered:

```markdown
## Platform Formatting
- **Telegram:** No markdown tables — use bullet lists. No headers — use **bold**.
```

## No Marker File Needed

These are permanent formatting rules, not a one-time setup step. Add them to your SOUL.md or AGENTS.md so they apply to every session automatically.
