#!/usr/bin/env python3
"""Emit the vinoa README hero banner as SVG, using only tokens.md palette values."""

from pathlib import Path

W, H = 1600, 600
BG = "#1e1e2e"       # tokens.md: DEFAULT_BG / Mocha Base
BORDER = "#585b70"   # tokens.md: border
FG = "#cdd6f4"       # tokens.md: fg
ACCENT = "#74c7ec"   # tokens.md: accent (current position only)
OK = "#a6e3a1"       # tokens.md: ok

parts = [
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
    f'<rect width="{W}" height="{H}" fill="{BG}"/>',
]

# --- Left: terminal frame with a file tree inside -------------------------
FX, FY, FW, FH, FR = 120, 110, 440, 380, 26
parts.append(
    f'<rect x="{FX}" y="{FY}" width="{FW}" height="{FH}" rx="{FR}" ry="{FR}" '
    f'fill="none" stroke="{BORDER}" stroke-width="2"/>'
)

trunk_x = 196
branches = [(215, 0), (290, 0), (365, 1)]
parts.append(
    f'<line x1="{trunk_x}" y1="188" x2="{trunk_x}" y2="412" '
    f'stroke="{BORDER}" stroke-width="2"/>'
)
for y, is_accent in branches:
    node_x, node = 306, 11
    parts.append(
        f'<line x1="{trunk_x}" y1="{y}" x2="{node_x - node}" y2="{y}" '
        f'stroke="{BORDER}" stroke-width="2"/>'
    )
    fill = ACCENT if is_accent else FG
    parts.append(
        f'<rect x="{node_x - node / 2}" y="{y - node / 2}" width="{node}" height="{node}" '
        f'rx="2" fill="{fill}"/>'
    )

# --- Checkmark tucked into the frame's lower-right corner -----------------
cx, cy = FX + FW - 6, FY + FH - 4
parts.append(
    f'<polyline points="{cx - 26},{cy - 8} {cx - 12},{cy + 6} {cx + 16},{cy - 28}" '
    f'fill="none" stroke="{OK}" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>'
)

# --- Right: version-matrix axis, density growing left to right ------------
AX, AY, AW = 700, 300, 780
parts.append(
    f'<line x1="{AX}" y1="{AY}" x2="{AX + AW}" y2="{AY}" '
    f'stroke="{BORDER}" stroke-width="2"/>'
)

COUNT, ACCENT_AT, SIZE = 26, 19, 12
for i in range(COUNT):
    t = (i / (COUNT - 1)) ** 1.7
    x = AX + AW * t
    fill = ACCENT if i == ACCENT_AT else FG
    parts.append(
        f'<rect x="{x - SIZE / 2:.1f}" y="{AY - SIZE / 2}" width="{SIZE}" height="{SIZE}" '
        f'rx="2" fill="{fill}"/>'
    )

# --- Quiet file-list column, far right, very low contrast -----------------
for i in range(6):
    y = 402 + i * 22
    w = 150 - i * 14
    parts.append(
        f'<rect x="1330" y="{y}" width="{w}" height="4" rx="2" fill="{BORDER}" opacity="0.55"/>'
    )

parts.append("</svg>")

out = Path("/root/vinoa/docs/brand/generated/vinoa-banner.svg")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text("\n".join(parts), encoding="utf-8")
print(f"wrote {out}")
