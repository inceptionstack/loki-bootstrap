# Draw.io Diagram Style Guide — AWS re:Invent Dark Theme

## Overview

This style guide defines a consistent, professional diagram style inspired by AWS re:Invent presentation decks. Use it for all architecture diagrams unless explicitly told otherwise.

**Output format: Always generate `.drawio` XML files** (draw.io / diagrams.net native XML format). Never output as SVG, PNG, Mermaid, PlantUML, or any other format. The `.drawio` XML is directly openable in draw.io desktop, app.diagrams.net, and VS Code with the draw.io extension.

---

## Canvas

| Property | Value |
|----------|-------|
| Background | `#0D1117` (GitHub dark) |
| Grid | Off (or very subtle) |
| Page | Borderless (`page="0"`) |
| Shadow | Off |

---

## Color Palette

### Lane / Category Colors

| Lane Purpose | Border / Accent | Fill (header pill) | Use for |
|-------------|----------------|-------------------|---------|
| Client / User | `#2EA043` (green) | Same as border | End-user facing components |
| Frontend / Edge | `#58A6FF` (blue) | Same as border | CDN, static hosting, edge |
| API / Compute | `#D29922` (amber) | Same as border | API Gateway, Lambda, EC2, ECS |
| Event / Async | `#F778BA` (pink) | Same as border | EventBridge, SQS, SNS, Step Functions |
| Data / Storage | `#BC8CFF` (purple) | Same as border | RDS, DynamoDB, S3 data stores |
| Security / Auth | `#F85149` (red) | Same as border | Cognito, IAM, WAF |

### Neutral Colors

| Element | Color |
|---------|-------|
| Primary text | `#F0F6FC` (white-ish) |
| Secondary text | `#8B949E` (grey) |
| Card/box fill | `#21262D` (dark card) |
| Card border | `#30363D` (subtle border) |
| Lane background fill | `#1A2332` (dark blue-grey) |
| Callout/legend fill | `#161B22` (darker card) |
| Danger/warning bg | `#3D1114` (dark red tint) |
| Success bg | `#0D2818` (dark green tint) |

---

## Lanes (Swim Lanes)

Vertical lanes group components by architectural concern.

### Lane Background
```
style="rounded=1;whiteSpace=wrap;fillColor=#1A2332;strokeColor=<LANE_COLOR>;strokeWidth=2;opacity=60;arcSize=4;"
```

### Lane Header (Pill Badge)
```
style="text;html=1;fontSize=16;fontStyle=1;fontColor=<LANE_COLOR>;align=center;
  fillColor=<LANE_COLOR>;rounded=1;arcSize=20;strokeColor=none;"
```
- ALL CAPS text (e.g., "CLIENT", "API LAYER", "DATA LAYER")
- Centered at top of lane
- Width: ~120–140px, height: 36px

---

## AWS Service Icons

Use the built-in `mxgraph.aws4` shape library. Always use `resourceIcon` style.

### Icon Template
```
style="shape=mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.<service>;
  labelBackgroundColor=none;sketch=0;fillColor=<SERVICE_COLOR>;strokeColor=none;"
```

### Standard Icon Sizes
- Primary service: **60×60**
- Secondary/badge: **40×40**
- Inline/small: **30×30**

### Common AWS Icon Colors

| Service | resIcon key | fillColor |
|---------|------------|-----------|
| S3 | `s3` | `#3F8624` |
| Lambda | `lambda` | `#ED7100` |
| EC2 | `ec2` | `#ED7100` |
| API Gateway | `api_gateway` | `#E7157B` |
| RDS | `rds` | `#C925D1` |
| DynamoDB | `dynamodb` | `#C925D1` |
| CloudFront | `cloudfront` | `#8C4FFF` |
| ELB / ALB | `elastic_load_balancing` | `#8C4FFF` |
| EventBridge | `eventbridge` | `#E7157B` |
| SQS | `sqs` | `#E7157B` |
| SNS | `sns` | `#E7157B` |
| Cognito | `cognito` | `#DD344C` |
| Step Functions | `step_functions` | `#E7157B` |
| ECS | `ecs` | `#ED7100` |
| Fargate | `fargate` | `#ED7100` |
| SageMaker | `sagemaker` | `#01A88D` |
| Secrets Manager | `secrets_manager` | `#DD344C` |
| CloudWatch | `cloudwatch` | `#E7157B` |
| CodeBuild | `codebuild` | `#C925D1` |
| CodePipeline | `codepipeline` | `#C925D1` |
| WAF | `waf` | `#DD344C` |
| Kinesis | `kinesis` | `#8C4FFF` |
| Mobile client | `mobile_client` | `#C7131F` |

---

## Labels

### Service Label (below icon)
```
style="text;html=1;fontSize=12;fontColor=#F0F6FC;align=center;
  fillColor=none;strokeColor=none;fontFamily=Arial;"
```
Format:
```html
<b>Service Name</b><br><font style="font-size:10px">Subtitle / description</font>
```

### Numbered Steps (inside grouped box)
```
style="text;html=1;fontSize=12;fontColor=#F0F6FC;align=left;
  fillColor=#21262D;strokeColor=#30363D;strokeWidth=1;rounded=1;arcSize=10;
  fontFamily=Arial;spacingLeft=10;spacingRight=10;spacing=4;"
```
Format:
```html
<font color="<LANE_COLOR>">①</font> <b>Step Name</b><br><font style="font-size:10px">Description</font>
```
Use circled numbers: ① ② ③ ④ ⑤ ⑥ ⑦ ⑧ ⑨ ⑩

---

## Grouping Boxes (Dashed Boundaries)

For logical clusters (e.g., "EC2 Monolith", "VPC", "Private Subnet"):

```
style="rounded=1;whiteSpace=wrap;fillColor=#1C2636;strokeColor=<LANE_COLOR>;
  strokeWidth=2;dashed=1;dashPattern=5 3;arcSize=6;"
```

Group label (inside, top):
```
style="text;html=1;fontSize=11;fontStyle=1;fontColor=<LANE_COLOR>;align=center;
  fillColor=none;strokeColor=none;fontFamily=Arial;"
```
- ALL CAPS (e.g., "EC2 MONOLITH", "PRIVATE SUBNET")

---

## Arrows / Connections

### Standard Flow Arrow
```
style="endArrow=block;endFill=1;strokeColor=<LANE_COLOR>;strokeWidth=2;rounded=1;"
```

### Return / Response Arrow (dashed)
```
style="endArrow=block;endFill=1;strokeColor=#F85149;strokeWidth=2;
  dashed=1;dashPattern=8 4;rounded=1;"
```

### Arrow Label
```
style="text;html=1;fontSize=9;fontColor=<ARROW_COLOR>;align=center;
  fillColor=none;strokeColor=none;fontFamily=Arial;"
```

### Color Convention
- Arrows match the **source lane color** or the **flow type**
- Green (`#2EA043` / `#3FB950`) — user/client flows
- Blue (`#58A6FF`) — frontend/edge flows
- Amber (`#D29922`) — compute/API flows
- Purple (`#BC8CFF`) — data read/write flows
- Pink (`#F778BA`) — async/event flows
- Red (`#F85149`) — return paths, errors, warnings

---

## Badges & Callouts

### Status Badge (small pill)
```
style="text;html=1;fontSize=9;fontStyle=1;fontColor=<COLOR>;align=center;
  fillColor=<TINTED_BG>;strokeColor=<COLOR>;strokeWidth=1;rounded=1;arcSize=20;
  fontFamily=Arial;spacingLeft=6;spacingRight=6;"
```

Examples:
- 🔒 RETAINED → green on dark green (`#2EA043` on `#0D2818`)
- ⚡ SYNCHRONOUS → red on dark red (`#F85149` on `#3D1114`)
- 🆕 NEW → blue on dark blue (`#58A6FF` on `#0D1B30`)
- ⚠️ DEPRECATED → amber on dark amber (`#D29922` on `#2D2000`)

### Annotation / Note Box
```
style="text;html=1;fontSize=10;fontColor=#8B949E;align=center;
  fillColor=#161B22;strokeColor=#30363D;strokeWidth=1;rounded=1;arcSize=10;
  fontFamily=Arial;spacing=4;"
```

### Legend / Callout Box
```
style="rounded=1;whiteSpace=wrap;fillColor=#161B22;strokeColor=#30363D;
  strokeWidth=1;arcSize=6;"
```
With bold title + bullet list in `#8B949E` secondary text.

---

## Title Block

Place top-right or top-left outside the lanes.

### Main Title
```
style="text;html=1;fontSize=24;fontStyle=1;fontColor=#F0F6FC;align=left;
  fillColor=none;strokeColor=none;fontFamily=Arial;"
```

### Subtitle
```
style="text;html=1;fontSize=13;fontColor=#8B949E;align=left;
  fillColor=none;strokeColor=none;fontFamily=Arial;"
```

---

## Layout Rules

1. **Flow direction**: Left-to-right (client → edge → compute → data) or top-to-bottom for sequential steps
2. **Lane width**: 200–340px depending on content density
3. **Lane gap**: 20px between lanes
4. **Icon spacing**: At least 40px vertical between service icons
5. **Step cards**: Full lane width minus 20px padding, 42px tall, 55px vertical spacing
6. **Minimum canvas**: 1100×800 for a 4-lane diagram
7. **Font**: Always `Arial` — renders everywhere

---

## XML Boilerplate

```xml
<mxfile host="app.diagrams.net">
  <diagram id="unique-id" name="Diagram Name">
    <mxGraphModel dx="1600" dy="1000" grid="1" gridSize="10"
      guides="1" tooltips="1" connect="1" arrows="1" fold="1"
      page="0" pageScale="1" pageWidth="1600" pageHeight="900"
      background="#0D1117" math="0" shadow="0">
      <root>
        <mxCell id="0" />
        <mxCell id="1" parent="0" />
        <!-- Lanes → Icons → Labels → Arrows → Legend -->
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>
```

---

## Ordering Convention

Build the XML in this order for readability:
1. Lane backgrounds + lane headers
2. Title block
3. Service icons + labels (per lane, top to bottom)
4. Grouping boxes (dashed boundaries)
5. Step cards (numbered, inside groups)
6. Badges
7. Arrows + arrow labels
8. Legend / callout boxes

---

## Don'ts

- ❌ No white or light backgrounds
- ❌ No default draw.io blue arrows
- ❌ No serif fonts
- ❌ No gradients on shapes
- ❌ No shadow effects
- ❌ No text directly on icons (use labels below)
- ❌ No more than 6 lanes — split into multiple diagrams
- ❌ No unlabeled arrows
