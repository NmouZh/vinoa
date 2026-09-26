#!/usr/bin/env python3
"""Render the Bilibili cover for the vinoa demo video.

Bilibili cover spec: 16:9, 1146x717 minimum, safe area keeps the lower-right
clear (duration badge) and the lower-left clear (danmaku toggle overlay).
Title is real vector text, not AI-rendered, so glyphs are always correct.
"""

from pathlib import Path

W, H = 1920, 1080
BG = "#1e1e2e"
BORDER = "#585b70"
FG = "#cdd6f4"
DIM = "#a6adc8"
FAINT = "#6c7086"
ACCENT = "#74c7ec"
OK = "#a6e3a1"

CJK = "Noto Sans CJK SC"
MONO = "Noto Sans Mono"

p = [
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
    f'<rect width="{W}" height="{H}" fill="{BG}"/>',
]

# --- 左侧主标题 ---------------------------------------------------------
LX = 150
p.append(f'<text x="{LX}" y="400" font-family="{CJK}" font-size="128" font-weight="900" '
         f'fill="{FG}">写插件</text>')
p.append(f'<text x="{LX}" y="560" font-family="{CJK}" font-size="128" font-weight="900" '
         f'fill="{FG}">开头那半小时</text>')

# 强调行：用 accent 标出「十秒」
p.append(f'<text x="{LX}" y="720" font-family="{CJK}" font-size="76" font-weight="700" '
         f'fill="{DIM}">现在只要</text>')
p.append(f'<text x="{LX + 320}" y="720" font-family="{CJK}" font-size="76" font-weight="900" '
         f'fill="{ACCENT}">十秒</text>')

# 底部工具名
p.append(f'<line x1="{LX}" y1="820" x2="{LX + 150}" y2="820" stroke="{ACCENT}" stroke-width="6"/>')
p.append(f'<text x="{LX + 180}" y="838" font-family="{MONO}" font-size="52" font-weight="700" '
         f'fill="{FG}">vinoa</text>')

# 副标题
p.append(f'<text x="{LX}" y="920" font-family="{CJK}" font-size="40" fill="{FAINT}">'
         f'Minecraft 插件工程脚手架 · 61 个版本 · 7 个平台</text>')

# --- 右侧终端示意（真实排版，不是装饰） ---------------------------------
TX, TY, TW, TH, TR = 1080, 250, 700, 520, 24
p.append(f'<rect x="{TX}" y="{TY}" width="{TW}" height="{TH}" rx="{TR}" ry="{TR}" '
         f'fill="none" stroke="{BORDER}" stroke-width="3"/>')

# 标题栏三圆点
for i in range(3):
    p.append(f'<circle cx="{TX + 44 + i * 34}" cy="{TY + 44}" r="9" fill="{BORDER}"/>')
p.append(f'<line x1="{TX}" y1="{TY + 80}" x2="{TX + TW}" y2="{TY + 80}" stroke="{BORDER}" stroke-width="2"/>')

# 终端内容
lines = [
    ("$", "vinoa init my-plugin", ACCENT),
    ("", "-m 1.21.11 --platform paper", DIM),
    ("", "", DIM),
    ("", "✓ 完成: ./my-plugin", OK),
    ("$", "cd my-plugin", ACCENT),
    ("", "./gradlew build", DIM),
    ("", "", DIM),
    ("", "BUILD SUCCESSFUL in 8s", OK),
]
ty = TY + 140
for prompt, text, color in lines:
    if prompt:
        p.append(f'<text x="{TX + 44}" y="{ty}" font-family="{MONO}" font-size="30" '
                 f'font-weight="700" fill="{ACCENT}">{prompt}</text>')
        p.append(f'<text x="{TX + 86}" y="{ty}" font-family="{MONO}" font-size="30" '
                 f'fill="{FG}">{text}</text>')
    elif text:
        x = TX + 86
        p.append(f'<text x="{x}" y="{ty}" font-family="{MONO}" font-size="30" '
                 f'fill="{color}">{text}</text>')
    ty += 52

# --- 右下角留白：B 站时长角标会压在这里，不放内容 -----------------------

out = Path("/root/vinoa/docs/brand/bilibili-cover.svg")
out.write_text("\n".join(p) + "\n</svg>", encoding="utf-8")
print(f"wrote {out}")
