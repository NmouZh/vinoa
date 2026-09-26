#!/usr/bin/env python3
"""Render README support graphics for vinoa, locked to tokens.md palette.

Outputs (docs/brand/):
  flow.svg/png      一条命令的流程示意，替代纯文字描述
  matrix.svg/png    版本矩阵跨度图，给「它解决什么」配图
"""

from pathlib import Path

BG = "#1e1e2e"       # tokens.md DEFAULT_BG
BORDER = "#585b70"   # tokens.md border
FG = "#cdd6f4"       # tokens.md fg
DIM = "#a6adc8"      # tokens.md dim
FAINT = "#6c7086"    # tokens.md faint
ACCENT = "#74c7ec"   # tokens.md accent
OK = "#a6e3a1"       # tokens.md ok

OUT = Path(__file__).resolve().parent

CJK = "Noto Sans CJK SC"
MONO = "Noto Sans Mono"


def esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


# ---------------------------------------------------------------- flow chart
def flow() -> str:
    W, H = 1200, 260
    p = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
         f'<rect width="{W}" height="{H}" fill="{BG}"/>']

    steps = [
        ("vinoa init", "选 MC 版本 + 平台", ACCENT),
        ("版本矩阵", "坐标 / Java / Gradle", FG),
        ("生成工程", "43 个文件落盘", FG),
        ("./gradlew build", "BUILD SUCCESSFUL", OK),
    ]
    box_w, box_h, gap = 240, 92, 60
    x0 = (W - (len(steps) * box_w + (len(steps) - 1) * gap)) / 2
    y0 = (H - box_h) / 2

    for i, (title, sub, color) in enumerate(steps):
        x = x0 + i * (box_w + gap)
        p.append(f'<rect x="{x:.0f}" y="{y0:.0f}" width="{box_w}" height="{box_h}" rx="12" '
                 f'fill="none" stroke="{BORDER}" stroke-width="2"/>')
        p.append(f'<text x="{x + box_w / 2:.0f}" y="{y0 + 38:.0f}" text-anchor="middle" '
                 f'font-family="{MONO}" font-size="19" font-weight="600" fill="{color}">{esc(title)}</text>')
        p.append(f'<text x="{x + box_w / 2:.0f}" y="{y0 + 66:.0f}" text-anchor="middle" '
                 f'font-family="{CJK}" font-size="14" fill="{DIM}">{esc(sub)}</text>')
        if i < len(steps) - 1:
            ax = x + box_w + 14
            bx = x + box_w + gap - 14
            mid = y0 + box_h / 2
            p.append(f'<line x1="{ax:.0f}" y1="{mid:.0f}" x2="{bx:.0f}" y2="{mid:.0f}" '
                     f'stroke="{BORDER}" stroke-width="2"/>')
            p.append(f'<polyline points="{bx - 9:.0f},{mid - 5:.0f} {bx:.0f},{mid:.0f} '
                     f'{bx - 9:.0f},{mid + 5:.0f}" fill="none" stroke="{BORDER}" stroke-width="2"/>')

    p.append("</svg>")
    return "\n".join(p)


# ------------------------------------------------------------ matrix span
def matrix() -> str:
    W, H = 1200, 300
    p = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">',
         f'<rect width="{W}" height="{H}" fill="{BG}"/>']

    ax, ay, aw = 110, 150, 980
    p.append(f'<line x1="{ax}" y1="{ay}" x2="{ax + aw}" y2="{ay}" stroke="{BORDER}" stroke-width="2"/>')

    # 61 个版本，节点密度递增，体现「版本跨度大、逐代变密」
    n, size = 61, 9
    accent_at = 60
    for i in range(n):
        t = (i / (n - 1)) ** 1.9
        x = ax + aw * t
        fill = ACCENT if i == accent_at else (FG if i % 5 == 0 else DIM)
        p.append(f'<rect x="{x - size / 2:.1f}" y="{ay - size / 2}" width="{size}" height="{size}" '
                 f'rx="2" fill="{fill}"/>')

    labels = [("1.8.9", 0.0), ("1.16.5", 0.30), ("1.21.11", 0.62), ("26.2", 1.0)]
    for text, t in labels:
        x = ax + aw * (t ** 1.9) if t < 1 else ax + aw
        p.append(f'<line x1="{x:.0f}" y1="{ay + 14}" x2="{x:.0f}" y2="{ay + 24}" stroke="{FAINT}" stroke-width="1.5"/>')
        anchor = "end" if t == 1.0 else ("start" if t == 0 else "middle")
        p.append(f'<text x="{x:.0f}" y="{ay + 44}" text-anchor="{anchor}" font-family="{MONO}" '
                 f'font-size="15" fill="{DIM}">{esc(text)}</text>')

    p.append(f'<text x="{ax}" y="{ay - 46}" font-family="{CJK}" font-size="17" fill="{FG}">'
             f'内置版本矩阵</text>')
    p.append(f'<text x="{ax}" y="{ay - 22}" font-family="{CJK}" font-size="14" fill="{FAINT}">'
             f'61 个 MC 版本 · 7 个平台 · 每条坐标对着上游仓库实测</text>')
    p.append(f'<text x="{ax + aw:.0f}" y="{ay - 46}" text-anchor="end" font-family="{MONO}" '
             f'font-size="15" fill="{OK}">Java 8 → 25</text>')

    p.append("</svg>")
    return "\n".join(p)


for name, builder in (("flow", flow), ("matrix", matrix)):
    svg = OUT / f"{name}.svg"
    svg.write_text(builder(), encoding="utf-8")
    print(f"wrote {svg}")
