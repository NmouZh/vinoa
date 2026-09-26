#!/usr/bin/env python3
"""Render the vinoa interface prototypes (and the token sheet) to PNG.

Prototypes are authored as a cell grid and emitted as ANSI, then rasterised by
`scripts/termcap/ansishot.py`. Authoring in the terminal's own medium (cells +
SGR) rather than as pictures keeps each prototype directly readable as an
implementation spec: the column arithmetic and the colour tokens are the spec.

The colour table below is derived from `docs/spec/ui/tokens.md` (issue #22):
`S` is built from the frozen 256-colour baseline so the nine already-accepted
prototypes render byte-for-byte identically, while `tstyle()` can also emit the
true-colour and 16-colour tiers for the token sheet.

Usage:
  python3 render.py                # writes every prototype next to this file
  python3 render.py p1             # just one
  python3 render.py tokens         # the token reference sheet (#22)
"""
import os
import subprocess
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
ANSHOT = os.path.join(ROOT, "scripts", "termcap", "ansishot.py")

COLS, ROWS = 100, 28

# ── colour tokens (docs/spec/ui/tokens.md §3.1) ──────────────────────────────
#
# Eight semantic slots are the complete set REC needs; `accentb` / `title` are
# weight variants, `sel` is deprecated (zero uses in the nine prototypes, kept
# for key compatibility) and `hdr` / `hdrb` exist only so the rejected P4
# prototype still reproduces. See tokens.md §3.7.
#
# Three tiers, same eight slots.  The 256 tier is FROZEN: it is what the nine
# already-accepted PNGs were rendered with, so changing it would invalidate
# human-reviewed evidence (tokens.md §3.3).  The true-colour tier is the
# un-quantised target each 256 grid point was standing in for — the Catppuccin
# Mocha role that ansishot.py's DEFAULT_BG (#1e1e2e) already commits us to
# (tokens.md §3.2).  The 16-colour tier keeps the hue family and picks the
# brightest member of it, because nearest-neighbour collapses ok/warn/err into
# the same white as fg (tokens.md §3.4).
SLOTS = {
    # slot:   (256 base,        24-bit fg SGR,       16-colour SGR)
    "fg":     (252, (205, 214, 244), 97),
    "dim":    (245, (166, 173, 200), 37),
    "faint":  (240, (108, 112, 134), 90),
    "border": (60,  (88, 91, 112),   90),
    "accent": (74,  (116, 199, 236), 96),
    "ok":     (114, (166, 227, 161), 92),
    "warn":   (179, (250, 179, 135), 93),
    "err":    (210, (243, 139, 168), 91),
    "sel":    (111, (137, 180, 250), 94),   # deprecated, see tokens.md §3.7
}
# P4-only background slots (rejected direction; kept for reproduction).
HDR_BG = {"hdr": (60, (69, 71, 90), 7), "hdrb": (60, (49, 50, 68), 7)}


def style(slot, tier="256", bold=False, bg=None):
    """Build an SGR fragment for one semantic slot.

    Order is always `bg -> bold -> fg` so one semantic never has two spellings.
    `tier` is "24" (true colour), "256" (indexed) or "16" (bare ANSI).
    """
    base, rgb, ansi = SLOTS[slot]
    if tier == "24":
        fg = "38;2;%d;%d;%d" % rgb
    elif tier == "16":
        fg = str(ansi)
    else:
        fg = "38;5;%d" % base
    parts = []
    if bg is not None:
        bbase, brgb, bansi = HDR_BG[bg]
        if tier == "16":
            # No colour blocks in the 16-colour tier: reverse video instead
            # (tokens.md §3.1).
            parts.append("7")
        elif tier == "24":
            parts.append("48;2;%d;%d;%d" % brgb)
        else:
            parts.append("48;5;%d" % bbase)
    if bold:
        parts.append("1")
    parts.append(fg)
    return ";".join(parts)


# The table the prototypes actually use.  Derived — not hardcoded — so that
# tokens.md stays the single source of truth and the emitted bytes stay
# identical to the pre-#22 hardcoded values.
S = {
    "fg": style("fg"),
    "dim": style("dim"),
    "faint": style("faint"),
    "border": style("border"),
    "accent": style("accent"),
    "accentb": style("accent", bold=True),
    "ok": style("ok"),
    "warn": style("warn"),
    "err": style("err"),
    "sel": style("sel", bold=True),
    "title": style("fg", bold=True),
    "hdr": style("fg", bg="hdr"),
    "hdrb": style("fg", bold=True, bg="hdrb"),
}
RESET = "\x1b[0m"

# ── glyphs (docs/spec/ui/tokens.md §4) ───────────────────────────────────────
# Every glyph has an ASCII fallback; the fallback is a separate axis from the
# colour tier (no-colour keeps the Unicode glyph and only drops colour).
GLYPH = {
    "cursor": "❯", "nav.done": "✓", "nav.current": "▸", "nav.future": "·",
    "nav.sep": "›", "check.on": "[x]", "check.off": "[ ]",
    "ok": "✓", "err": "✗", "warn": "!", "sep": "·",
    "tree.tee": "├─", "tree.last": "└─", "tree.pipe": "│",
    "box.round": "╭╮╰╯", "box.square": "┌┐└┘",
    "bar.full": "█", "bar.empty": "░", "caret": "▏",
    # Multi-frame sequences (tokens.md §4).  Equal width per frame, so the
    # cycle cannot shift columns.
    "spin.frames.unicode": "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏",
    "spin.frames.ascii": "|/-\\",
}
GLYPH_ASCII = {
    "cursor": ">", "nav.done": "+", "nav.current": ">", "nav.future": "-",
    "nav.sep": ">", "check.on": "[x]", "check.off": "[ ]",
    "ok": "+", "err": "x", "warn": "!", "sep": "-",
    "tree.tee": "|-", "tree.last": "`-", "tree.pipe": "|",
    "box.round": "++++", "box.square": "++++",
    "bar.full": "#", "bar.empty": "-", "caret": "|",
    # The fallback is whole-group, never per-frame: ASCII tier must not emit
    # braille (tokens.md §4).
    "spin.frames.unicode": "|/-\\", "spin.frames.ascii": "|/-\\",
}


def width(ch):
    if unicodedata.combining(ch):
        return 0
    return 2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1


def dwidth(s):
    return sum(width(c) for c in s)


def pad(s, n):
    """Left-align `s` in a field `n` display columns wide."""
    return s + " " * max(0, n - dwidth(s))


class Canvas:
    def __init__(self, cols=COLS, rows=ROWS):
        self.cols, self.rows = cols, rows
        self.cells = [[(" ", None) for _ in range(cols)] for _ in range(rows)]

    def put(self, r, c, ch, style):
        if not (0 <= r < self.rows and 0 <= c < self.cols):
            return
        self.cells[r][c] = (ch, style)

    def text(self, r, c, s, style=None, limit=None):
        """Write `s` at (r, c); returns the next free column."""
        stop = self.cols if limit is None else min(self.cols, c + limit)
        for ch in s:
            w = width(ch)
            if w == 0:
                continue
            if c + w > stop:
                break
            self.put(r, c, ch, style)
            if w == 2:
                self.put(r, c + 1, "", style)
            c += w
        return c

    def rtext(self, r, c, s, style=None):
        """Right-align `s` so it ends at column c (inclusive)."""
        self.text(r, c - dwidth(s) + 1, s, style)

    def fill(self, r, c, n, ch, style):
        for i in range(n):
            self.put(r, c + i, ch, style)

    def box(self, r0, c0, w, h, style, title=None, title_style=None):
        """Draw a rounded frame of w x h at (r0, c0)."""
        top = "╭" + "─" * (w - 2) + "╮"
        self.text(r0, c0, top, style)
        if title:
            self.text(r0, c0 + 2, " " + title + " ", title_style or style)
        for r in range(r0 + 1, r0 + h - 1):
            self.put(r, c0, "│", style)
            self.put(r, c0 + w - 1, "│", style)
        self.text(r0 + h - 1, c0, "╰" + "─" * (w - 2) + "╯", style)

    def hsep(self, r, c0, w, style, tee=True):
        if tee:
            self.text(r, c0, "├" + "─" * (w - 2) + "┤", style)
        else:
            self.text(r, c0, "─" * w, style)

    def vline(self, r0, c, h, style):
        for r in range(r0, r0 + h):
            self.put(r, c, "│", style)

    def to_ansi(self):
        out = []
        for row in self.cells:
            line, cur = [], None
            for ch, style in row:
                if ch == "":
                    continue
                if style != cur:
                    line.append(RESET if style is None else "\x1b[" + style + "m")
                    cur = style
                line.append(ch)
            line.append(RESET)
            out.append("".join(line).rstrip())
        # CRLF: a real terminal's ONLCR turns "\n" into "\r\n", and ansishot's
        # parser (like any VT) only returns to column 0 on an explicit "\r".
        # No trailing newline: on the last row it would scroll and eat row 0.
        return "\r\n".join(out)


# ── P1 · 克制专业 ───────────────────────────────────────────────────────────
def p1():
    """Restrained IDE: the spec §4 frame, semantic colour only."""
    cv = Canvas()
    W, H = 100, 24
    cv.box(0, 0, W, H, S["border"], "vinoa · 新建工程", S["title"])
    cv.text(1, 2, "① 工程", S["dim"])
    cv.text(1, 10, "›", S["faint"])
    cv.text(1, 12, "② 构建", S["dim"])
    cv.text(1, 20, "›", S["faint"])
    cv.text(1, 22, "③ 目标服务端", S["accentb"])
    cv.text(1, 36, "›", S["faint"])
    cv.text(1, 38, "④ 附加", S["dim"])
    cv.rtext(1, W - 4, "3/4", S["faint"])
    cv.hsep(2, 0, W, S["border"])

    cv.text(4, 4, "MC 版本", S["dim"])
    cv.text(4, 16, "1.21.11", S["fg"])
    cv.rtext(4, W - 6, "✓ 已选", S["ok"])
    cv.text(5, 4, "元数据", S["dim"])
    cv.text(5, 16, "plugin.yml", S["fg"])
    cv.rtext(5, W - 6, "✓ 已选", S["ok"])

    cv.text(7, 4, "平台（勾 paper 自动带上 bukkit）", S["dim"])
    rows = [
        ("paper", "Java 21", "io.papermc.paper:paper-api", True, True),
        ("bukkit", "Java 21", "org.spigotmc:spigot-api", True, False),
        ("velocity", "Java 25", "com.velocitypowered:velocity-api", False, False),
        ("bungeecord", "Java 8", "net.md-5:bungeecord-api", False, False),
        ("folia", "Java 21", "实验性", False, False),
        ("sponge", "Java 21", "实验性", False, False),
        ("minestom", "Java 25", "实验性", False, False),
    ]
    for i, (name, java, coord, on, cursor) in enumerate(rows):
        r = 9 + i
        cv.text(r, 4, "❯" if cursor else " ", S["accent"])
        cv.text(r, 6, "[x]" if on else "[ ]", S["ok"] if on else S["faint"])
        cv.text(r, 10, f"{name:<11}", S["fg"] if on else S["dim"])
        cv.text(r, 22, f"{java:<8}", S["dim"])
        cv.text(r, 31, coord, S["faint"])

    cv.text(17, 4, "环境预检", S["dim"])
    cv.text(17, 16, "Java 21  ✓  /usr/lib/jvm/java-21-openjdk", S["ok"])
    cv.text(18, 16, "Java 25  ✗  未检测到", S["err"])
    cv.text(19, 16, "→ 生成物 settings.gradle.kts 启用 foojay 自动下载？", S["warn"])

    cv.hsep(21, 0, W, S["border"])
    cv.text(22, 2, "↑↓ 移动   空格 勾选   → 全选   Enter 确认   ← → 切页   Esc 取消", S["dim"])
    return cv


# ── P2 · 分步导航 ───────────────────────────────────────────────────────────
def p2():
    """Step rail: the flow is the navigation; answered pages collapse to a gist."""
    cv = Canvas()
    cv.box(0, 0, 100, 24, S["border"])
    cv.vline(1, 26, 22, S["border"])
    cv.text(1, 2, "新建工程", S["title"])
    cv.text(3, 2, "✓", S["ok"])
    cv.text(5, 2, "① 工程", S["dim"])
    cv.text(6, 5, "my-plugin", S["faint"])
    cv.text(8, 2, "✓", S["ok"])
    cv.text(10, 2, "② 构建", S["dim"])
    cv.text(11, 5, "Java · Gradle", S["faint"])
    cv.text(13, 2, "▸", S["accent"])
    cv.text(15, 2, "③ 目标服务端", S["accentb"])
    cv.text(18, 2, "④ 附加", S["faint"])
    cv.text(22, 2, "Esc 取消", S["faint"])

    c = 28
    cv.text(1, c, "目标服务端", S["title"])
    cv.rtext(1, 98, "3/4", S["faint"])
    cv.text(3, c, "MC 版本", S["dim"])
    cv.text(4, c, "❯", S["accent"])
    cv.text(6, c, "1.21.11", S["fg"])
    cv.rtext(4, 98, "最新稳定", S["faint"])
    cv.text(5, c + 2, "1.21.10", S["faint"])
    cv.text(6, c + 2, "1.21.9", S["faint"])

    cv.text(8, c, "平台", S["dim"])
    cv.text(9, c, "❯ [x] paper", S["fg"])
    cv.text(9, c + 14, "[x] bukkit", S["fg"])
    cv.text(10, c + 2, "[ ] velocity", S["dim"])
    cv.text(10, c + 16, "[ ] bungeecord", S["dim"])
    cv.text(11, c + 2, "[ ] folia", S["dim"])
    cv.text(11, c + 16, "[ ] sponge", S["dim"])
    cv.text(11, c + 30, "[ ] minestom", S["dim"])

    cv.text(13, c, "环境预检", S["dim"])
    cv.text(14, c, "Java 21  ✓", S["ok"])
    cv.text(14, c + 12, "Java 25  ✗", S["err"])
    cv.text(15, c, "缺 Java 25：生成物启用 foojay 自动下载？", S["warn"])

    cv.hsep(17, 27, 72, S["border"])
    cv.text(19, c, "Enter 确认", S["dim"])
    cv.text(19, c + 14, "← → 切页", S["dim"])
    cv.text(19, c + 28, "空格 勾选", S["dim"])
    cv.hsep(21, 0, 100, S["border"])
    cv.text(22, 2, "vinoa init · my-plugin", S["faint"])
    cv.rtext(22, 98, "com.example.myplugin", S["faint"])
    return cv


# ── P3 · 安静日志 ───────────────────────────────────────────────────────────
def p3():
    """Quiet log: no chrome at all; beautiful by subtraction. gh/cargo register."""
    cv = Canvas()
    cv.text(0, 2, "vinoa", S["title"])
    cv.text(0, 8, "新建工程", S["faint"])
    cv.text(0, 18, "my-plugin", S["dim"])

    cv.text(2, 2, "✓", S["ok"])
    cv.text(2, 4, "① 工程", S["dim"])
    cv.text(2, 12, "my-plugin", S["fg"])
    cv.text(2, 23, "·", S["faint"])
    cv.text(2, 25, "com.example.myplugin", S["faint"])
    cv.text(2, 48, "·", S["faint"])
    cv.text(2, 50, "./my-plugin", S["faint"])

    cv.text(3, 2, "✓", S["ok"])
    cv.text(3, 4, "② 构建", S["dim"])
    cv.text(3, 12, "Java", S["fg"])
    cv.text(3, 18, "·", S["faint"])
    cv.text(3, 20, "Gradle", S["faint"])
    cv.text(3, 28, "·", S["faint"])
    cv.text(3, 30, "Kotlin DSL", S["faint"])
    cv.text(3, 42, "·", S["faint"])
    cv.text(3, 44, "JDK 21", S["faint"])

    cv.text(4, 2, "●", S["accent"])
    cv.text(4, 4, "③ 目标服务端", S["accentb"])

    cv.text(6, 6, "MC 版本", S["dim"])
    cv.text(7, 6, "❯", S["accent"])
    cv.text(7, 8, "1.21.11", S["fg"])
    cv.rtext(7, 40, "最新稳定", S["faint"])
    cv.text(8, 8, "1.21.10", S["faint"])
    cv.text(9, 8, "1.21.9", S["faint"])

    cv.text(11, 6, "平台", S["dim"])
    cv.text(12, 6, "❯", S["accent"])
    cv.text(12, 8, "[x] paper", S["fg"])
    cv.text(12, 20, "[x] bukkit", S["fg"])
    cv.text(12, 32, "[ ] velocity", S["dim"])
    cv.text(12, 46, "[ ] bungeecord", S["dim"])
    cv.text(13, 8, "[ ] folia", S["dim"])
    cv.text(13, 20, "[ ] sponge", S["dim"])
    cv.text(13, 32, "[ ] minestom", S["dim"])
    cv.text(14, 8, "paper 会自动带上 bukkit 兼容模块", S["faint"])

    cv.text(16, 6, "环境预检", S["dim"])
    cv.text(17, 8, "✓ Java 21", S["ok"])
    cv.text(17, 20, "/usr/lib/jvm/java-21-openjdk", S["faint"])
    cv.text(18, 8, "✗ Java 25", S["err"])
    cv.text(18, 20, "未检测到", S["faint"])
    cv.text(19, 8, "缺 Java 25：是否让 Gradle 首次构建自动下载？", S["warn"])
    cv.text(19, 58, "(Y/n)", S["fg"])

    cv.text(22, 2, "↑↓ 选择", S["dim"])
    cv.text(22, 12, "·", S["faint"])
    cv.text(22, 14, "空格 勾选", S["dim"])
    cv.text(22, 26, "·", S["faint"])
    cv.text(22, 28, "Enter 确认", S["dim"])
    cv.text(22, 41, "·", S["faint"])
    cv.text(22, 43, "Esc 取消", S["dim"])
    return cv


# ── P4 · 面板质感 ───────────────────────────────────────────────────────────
def p4():
    """Panel + live preview: heavy chrome, status bar, artifact tree as you answer."""
    cv = Canvas()
    W, H = 100, 26
    cv.box(0, 0, W, H, S["border"])
    cv.text(0, 2, " vinoa ", S["hdrb"])
    cv.text(0, 9, " my-plugin ", S["hdr"])
    cv.rtext(0, 78, " 就绪 ", S["hdr"])
    cv.hsep(1, 0, W, S["border"])
    cv.vline(2, 52, 20, S["border"])
    cv.hsep(22, 0, W, S["border"])

    # left: the form
    cv.text(2, 2, "③ 目标服务端", S["accentb"])
    cv.rtext(2, 50, "3/4", S["faint"])
    cv.text(4, 2, "MC 版本", S["dim"])
    cv.text(5, 2, "❯ 1.21.11", S["fg"])
    cv.rtext(5, 50, "最新稳定", S["faint"])
    cv.text(6, 4, "1.21.10", S["faint"])
    cv.text(7, 4, "1.21.9", S["faint"])

    cv.text(9, 2, "平台", S["dim"])
    cv.text(10, 2, "❯ [x] paper", S["fg"])
    cv.rtext(10, 50, "Java 21", S["faint"])
    cv.text(11, 4, "[x] bukkit", S["fg"])
    cv.rtext(11, 50, "Java 21", S["faint"])
    cv.text(12, 4, "[ ] velocity", S["dim"])
    cv.rtext(12, 50, "Java 25", S["faint"])
    cv.text(13, 4, "[ ] bungeecord", S["dim"])
    cv.rtext(13, 50, "Java 8", S["faint"])
    cv.text(14, 4, "[ ] folia", S["dim"])
    cv.rtext(14, 50, "实验性", S["faint"])
    cv.text(15, 4, "[ ] sponge", S["dim"])
    cv.rtext(15, 50, "实验性", S["faint"])
    cv.text(16, 4, "[ ] minestom", S["dim"])
    cv.rtext(16, 50, "实验性", S["faint"])

    cv.text(18, 2, "环境预检", S["dim"])
    cv.text(19, 2, "Java 21  ✓", S["ok"])
    cv.text(19, 14, "Java 25  ✗", S["err"])

    # right: live preview of what will be written
    cv.text(2, 54, "即将生成", S["dim"])
    cv.rtext(2, 98, "43 个文件", S["accentb"])
    cv.text(4, 54, "my-plugin/", S["fg"])
    tree = [
        ("├─ settings.gradle.kts", "faint"),
        ("├─ build.gradle.kts", "faint"),
        ("├─ core/", "fg"),
        ("│    └─ 9 files", "faint"),
        ("├─ platforms/", "fg"),
        ("│    ├─ paper/    7 files", "faint"),
        ("│    └─ bukkit/   6 files", "faint"),
        ("├─ gradle/", "fg"),
        ("└─ gradlew", "faint"),
    ]
    for i, (line, style) in enumerate(tree):
        cv.text(5 + i, 54, line, S[style])

    cv.text(15, 54, "跳过 16 项", S["dim"])
    cv.text(16, 54, "· README.en.md", S["faint"])
    cv.text(17, 54, "· paper-plugin.yml", S["faint"])
    cv.text(18, 54, "· 5 个未勾选模块", S["faint"])

    cv.text(20, 54, "git init + 首次提交", S["dim"])
    cv.rtext(20, 98, "[x]", S["ok"])

    cv.text(23, 2, "↑↓ 移动   空格 勾选   → 全选   Enter 确认   ← → 切页   Esc 取消", S["dim"])
    cv.rtext(23, 98, "1.21.11 · paper+bukkit", S["faint"])
    return cv


# ── SHELL · 长任务 / 错误 / 完成 ────────────────────────────────────────────
def shell():
    """The surfaces where vinoa currently prints nothing, or prints a wall.

    Three stacked exhibits in the P1 visual language: a determinate long task
    (the `--verify` gradle build, minutes of dead air today), the error path
    with the version wall folded away, and the completion banner.
    """
    cv = Canvas(COLS, 30)

    # ── exhibit A: long task ────────────────────────────────────────────────
    cv.box(0, 0, 100, 6, S["border"])
    cv.text(1, 3, "验证构建", S["dim"])
    cv.rtext(1, 97, "00:42", S["faint"])
    cv.text(2, 3, "█" * 22, S["accent"])
    cv.text(2, 25, "░" * 18, S["faint"])
    cv.text(2, 45, "55%", S["accentb"])
    cv.text(3, 3, "./gradlew build --offline", S["faint"])
    cv.text(4, 3, "› Task :platforms:paper:compileJava", S["fg"])
    cv.text(4, 60, "已完成 128 / 233", S["faint"])

    # ── exhibit B: error ───────────────────────────────────────────────────
    cv.box(8, 0, 100, 9, S["border"])
    cv.text(9, 3, "✗", S["err"])
    cv.text(9, 5, "平台与版本组合不存在：", S["fg"])
    cv.text(9, 27, "paper × 1.8.9", S["err"])

    cv.text(11, 3, "原因", S["dim"])
    cv.text(11, 10, "Paper 最早的构建是 1.8.8，1.8.9 没有 paper-api 制品", S["fg"])

    cv.text(12, 3, "1.8.9 可用平台", S["dim"])
    cv.text(12, 20, "bukkit", S["ok"])
    cv.text(12, 28, "sponge", S["ok"])

    cv.text(13, 3, "paper 可用版本", S["dim"])
    cv.text(13, 20, "1.9.4 … 1.12.2", S["fg"])
    cv.text(13, 34, "（共 51 个）", S["faint"])
    cv.rtext(13, 97, "vinoa versions --matrix --mc 1.8.9", S["faint"])

    cv.text(15, 3, "复现", S["dim"])
    cv.text(15, 10, "vinoa init my-plugin -m 1.8.9 --platform bukkit -y", S["fg"])

    # ── exhibit C: done ────────────────────────────────────────────────────
    cv.box(18, 0, 100, 8, S["border"])
    cv.text(19, 3, "✓", S["ok"])
    cv.text(19, 5, "完成", S["fg"])
    cv.text(19, 12, "my-plugin", S["title"])
    cv.text(19, 22, "43 个文件 · 0.3s", S["faint"])
    cv.text(21, 3, "cd my-plugin", S["fg"])
    cv.text(22, 3, "./gradlew build", S["accentb"])
    cv.text(22, 20, "# 构建插件 jar", S["faint"])
    cv.text(23, 3, "./gradlew runServer", S["fg"])
    cv.text(23, 24, "# 起本地测试服（首次需联网）", S["faint"])
    cv.text(24, 3, "注意: 目标 1.21.11 需要 Java 21", S["warn"])
    cv.rtext(24, 97, "docs/README.md · paper 章节", S["faint"])
    return cv


# ── REC · 推荐方向（P1 骨架 + P3 减法 + P4 预览） ──────────────────────────
PAGES = ["工程", "构建", "目标服务端", "附加"]
RIGHT_C = 66          # column of the preview divider
RW = 100


def _rec_shell(cv, page):
    """Frame, page nav, preview divider, hint bar. Shared by all four pages."""
    cv.box(0, 0, RW, 25, S["border"], "vinoa · 新建工程", S["title"])
    cv.rtext(0, 96, " %s " % PAGES[page - 1], S["hdr"])

    # nav: done = ok, current = accent, future = faint (no repeated page title)
    c = 2
    for i, name in enumerate(PAGES, start=1):
        mark = "✓" if i < page else ("▸" if i == page else "·")
        style = S["ok"] if i < page else (S["accentb"] if i == page else S["faint"])
        cv.text(1, c, mark, style)
        c = cv.text(1, c + 2, "%d %s" % (i, name), style) + 2
        if i < 4:
            cv.text(1, c, "›", S["faint"])
            c += 3
    cv.rtext(1, 97, "%d/4" % page, S["faint"])
    cv.hsep(2, 0, RW, S["border"])

    cv.vline(3, RIGHT_C, 19, S["border"])
    cv.hsep(22, 0, RW, S["border"])
    cv.text(23, 2, "↑↓ 移动", S["dim"])
    for col, label in ((10, "空格 勾选"), (22, "Enter 确认"), (35, "← → 切页"), (48, "Esc 取消")):
        cv.text(23, col - 2, "·", S["faint"])
        cv.text(23, col, label, S["dim"])
    return cv


def _rec_preview(cv, rows, count=None):
    """Right-hand live preview. `rows` are (indent, text, style) triples.

    `count` is only passed once the plan is actually determined (page ③+):
    promising a file count on page ① would be a lie, since the platform
    choice is what fixes the file set.
    """
    cv.text(3, RIGHT_C + 2, "即将生成", S["dim"])
    if count:
        cv.rtext(3, 97, count, S["accentb"])
    cv.text(4, RIGHT_C + 1, "─" * 31, S["border"])
    for i, (indent, text, style) in enumerate(rows):
        cv.text(5 + i, RIGHT_C + 2 + indent, text, S[style])


def rec(page):
    cv = _rec_shell(Canvas(RW, 25), page)

    if page == 1:
        cv.text(4, 3, "名称", S["dim"])
        cv.text(5, 3, "❯", S["accent"])
        cv.text(5, 5, "my-plugin", S["fg"])
        cv.put(5, 15, "▏", S["accent"])
        cv.text(7, 3, "包名", S["dim"])
        cv.text(8, 3, "❯", S["accent"])
        cv.text(8, 5, "com.example.myplugin", S["fg"])
        cv.text(10, 3, "位置", S["dim"])
        cv.text(11, 3, "  ./my-plugin", S["faint"])
        cv.text(11, 20, "（可写）", S["faint"])
        cv.text(14, 3, "↑↓ 或直接输入", S["faint"])
        cv.text(15, 3, "须匹配 ^[A-Za-z][A-Za-z0-9_-]{0,63}$", S["faint"])
        _rec_preview(cv, [
            (0, "my-plugin/", "fg"),
            (0, "└─ （选择平台后显示）", "faint"),
        ])

    elif page == 2:
        cv.text(4, 3, "语言", S["dim"])
        cv.text(4, 16, "Java", S["fg"])
        cv.text(6, 3, "构建系统", S["dim"])
        cv.text(6, 16, "Gradle", S["fg"])
        cv.text(8, 3, "Gradle DSL", S["dim"])
        cv.text(8, 16, "Kotlin (build.gradle.kts)", S["fg"])
        cv.text(11, 3, "JDK", S["dim"])
        cv.text(11, 16, "21", S["fg"])
        cv.rtext(11, 63, "由 MC 版本经矩阵推导", S["faint"])
        cv.text(13, 3, "本页无可选项：v1 每项只有一个取值。", S["faint"])
        cv.text(14, 3, "Gradle DSL 只有 Kotlin —— Paper 官方文档只覆盖它。", S["faint"])
        cv.text(15, 3, "Java 版本不在这里问——它由 ③ 选的 MC 版本推出。", S["faint"])
        _rec_preview(cv, [
            (0, "build.gradle.kts", "faint"),
            (0, "gradle/libs.versions.toml", "faint"),
            (0, "settings.gradle.kts", "faint"),
        ])

    elif page == 3:
        cv.text(4, 3, "MC 版本", S["dim"])
        cv.rtext(4, 63, "只列受支持的版本", S["faint"])
        cv.text(5, 3, "❯", S["accent"])
        cv.text(5, 5, "1.21.11", S["fg"])
        cv.rtext(5, 63, "最新稳定", S["faint"])
        cv.text(6, 7, "1.21.10", S["faint"])
        cv.text(7, 7, "1.21.9", S["faint"])

        cv.text(9, 3, "平台", S["dim"])
        cv.rtext(9, 63, "勾 paper 自动带上 bukkit", S["faint"])
        plats = [("paper", "Java 21", True, True), ("bukkit", "Java 21", True, False),
                 ("velocity", "Java 25", False, False), ("bungeecord", "Java 8", False, False),
                 ("folia", "Java 21", False, False), ("sponge", "Java 21", False, False),
                 ("minestom", "Java 25", False, False)]
        for i, (n, j, on, cur) in enumerate(plats):
            r = 10 + i
            cv.text(r, 3, "❯" if cur else " ", S["accent"])
            cv.text(r, 5, "[x]" if on else "[ ]", S["ok"] if on else S["faint"])
            cv.text(r, 9, "%-11s" % n, S["fg"] if on else S["dim"])
            cv.rtext(r, 63, j, S["faint"])

        cv.text(18, 3, "环境预检", S["dim"])
        cv.text(19, 5, "✓ Java 21", S["ok"])
        cv.text(19, 16, "/usr/lib/jvm/java-21-openjdk", S["faint"])
        cv.text(20, 5, "✗ Java 25", S["err"])
        cv.text(20, 16, "未检测到 · 生成物启用 foojay 自动下载？", S["warn"])

        _rec_preview(cv, [
            (0, "my-plugin/", "fg"),
            (0, "├─ settings.gradle.kts", "faint"),
            (0, "├─ build.gradle.kts", "faint"),
            (0, "├─ core/            9 files", "faint"),
            (0, "├─ platforms/", "fg"),
            (0, "│   ├─ paper/       7 files", "faint"),
            (0, "│   └─ bukkit/      6 files", "faint"),
            (0, "└─ gradlew", "faint"),
            (0, "", "faint"),
            (0, "跳过 16 项", "dim"),
            (0, "· README.en.md", "faint"),
            (0, "· paper-plugin.yml", "faint"),
        ], count="43 个文件")

    else:
        cv.text(4, 3, "附加模块", S["dim"])
        cv.rtext(4, 63, "默认全不勾", S["faint"])
        feats = ["SQLite 持久化", "bStats 统计", "更新检查", "PlaceholderAPI", "GUI 菜单骨架"]
        for i, f in enumerate(feats):
            cv.text(5 + i, 5, "[ ]", S["faint"])
            cv.text(5 + i, 9, f, S["dim"])
        cv.text(11, 3, "示例代码", S["dim"])
        cv.rtext(11, 63, "(Y/n) 默认 Y", S["faint"])
        cv.text(12, 3, "权限声明", S["dim"])
        cv.rtext(12, 63, "(Y/n) 默认 Y", S["faint"])
        cv.text(13, 3, "质量工程", S["dim"])
        cv.rtext(13, 63, "(Y/n) 默认 Y", S["faint"])
        cv.text(14, 3, "git init + 首次提交", S["dim"])
        cv.rtext(14, 63, "(Y/n) 默认 Y", S["faint"])
        cv.text(16, 3, "生成物语言", S["dim"])
        cv.text(16, 16, "❯ 中文", S["accent"])
        cv.text(16, 24, "English", S["dim"])
        cv.text(16, 34, "双语", S["dim"])
        cv.text(18, 3, "本页勾 bStats 会追问数字 plugin id。", S["faint"])
        cv.text(19, 3, "生成物语言 ≠ 界面语言。", S["faint"])
        _rec_preview(cv, [
            (0, "已选", "dim"),
            (0, "· 示例代码", "faint"),
            (0, "· 权限声明", "faint"),
            (0, "· 质量工程", "faint"),
            (0, "· git init", "faint"),
            (0, "", "faint"),
            (0, "语言", "dim"),
            (0, "· zh（README 中文）", "faint"),
        ], count="43 个文件")
    return cv


def rec1():
    return rec(1)


def rec2():
    return rec(2)


def rec3():
    return rec(3)


def rec4():
    return rec(4)


# ── TOKENS · token 表样例（#22） ────────────────────────────────────────────
# Not a product screen: a reference sheet for docs/spec/ui/tokens.md. Renders
# the three colour tiers side by side, the no-colour fallback for the same
# screen, the glyph set with its ASCII fallbacks, and the spacing/hierarchy
# numbers. Everything here is derived from the tables above, so the sheet
# cannot drift from the spec.
TOKEN_SLOTS = ["fg", "dim", "faint", "border", "accent", "ok", "warn", "err", "sel"]

# Everything at or right of this column is emitted with style=None, so the
# `.ans` contains no SGR for it. tokens.md §7 asserts exactly that, and #25
# turns the assertion into a test — so it has to be literally true, not just
# "true of the semantic colours".
NOCOLOR_COL = 52

# tokens.md §3.5 — what carries each slot once colour is gone.
NOCOLOR = {
    "fg": "（默认前景，无替代）",
    "dim": "缩进 +1 级",
    "faint": "缩进 +1 级 + · 前缀",
    "border": "保留框线字形",
    "accent": "当前项 ❯ / 导航 ▸",
    "ok": "行首 ✓",
    "warn": "行首 !",
    "err": "行首 ✗",
    "sel": "同 accent（deprecated）",
}

GLYPH_ORDER = [
    "cursor", "nav.done", "nav.current", "nav.future", "nav.sep",
    "check.on", "check.off", "ok", "err", "warn",
    "sep", "tree.tee", "tree.last", "tree.pipe", "box.round",
    "box.square", "bar.full", "bar.empty", "caret",
]
GLYPH_NOTE = {
    "cursor": "当前项，独占标记列", "nav.done": "导航·已完成", "nav.current": "导航·当前",
    "nav.future": "导航·未来页", "nav.sep": "导航页位分隔",
    "check.on": "多选·已勾", "check.off": "多选·未勾",
    "ok": "成功勾", "err": "失败叉（必须带）", "warn": "警告",
    "sep": "行内次要分隔", "tree.tee": "树·枝", "tree.last": "树·末",
    "tree.pipe": "树·竖线", "box.round": "框·圆角（默认）",
    "box.square": "框·直角（UTF-8）", "bar.full": "进度条·满",
    "bar.empty": "进度条·空", "caret": "输入光标",
}

SPACING = [
    ("screen.w.min", "100", "REC 完整形态最小宽度；降级阈值见 direction.md §4"),
    ("pad.frame", "2", "框线与内容之间的内边距（框线 col 0/99，内容 3–97）"),
    ("label.w", "13", "两列形态：标签 col 3，取值 col 16"),
    ("indent.unit", "2", "每级缩进 2 格；正文最多 2 级（col 3 → 5 → 7）"),
    ("col.divider", "66", "分栏竖线；表单 3–63，预览 68–97（比例 61:30 ≈ 2:1）"),
    ("box.h", "25", "整屏框高度（含上下框线）"),
]

HIERARCHY = [
    ("L1 标题", "title = fg+bold", "bold", "框线 / row.nav，独占"),
    ("L2 正文", "fg", "regular", "取值列 col 16（内联则 col 5）"),
    ("L3 次要", "dim", "regular", "标签列 col 3 / 右对齐 col 63·97"),
    ("L4 极次要", "faint", "regular", "缩进 +1 级，或 · 前缀"),
]


def _hex_of(slot):
    r, g, b = SLOTS[slot][1]
    return "#%02x%02x%02x" % (r, g, b)


def tokens():
    """The token sheet: three colour tiers, no-colour fallback, glyphs, spacing."""
    cv = Canvas(COLS, 60)
    cv.text(0, 0, "─" * 100, S["border"])
    cv.text(1, 2, "vinoa 设计 token 表", S["title"])
    cv.text(1, 24, "docs/spec/ui/tokens.md", S["faint"])
    cv.rtext(1, 97, "bg #1e1e2e", S["faint"])

    # ── A · colour, three tiers + no-colour ─────────────────────────────────
    cv.text(3, 2, "A · 色值：三档取值 + 无色降级", S["title"])
    cv.text(4, 3, pad("slot", 12), S["dim"])
    for c, label in ((16, "24-bit 真彩"), (32, "256 色"), (44, "16 色"), (54, "无色替代")):
        cv.text(4, c, label, S["dim"])
    for i, slot in enumerate(TOKEN_SLOTS):
        r = 5 + i
        cv.text(r, 3, pad(slot, 12), S["dim"])
        for col, tier in ((16, "24"), (32, "256"), (44, "16")):
            cv.text(r, col, "██", style(slot, tier))
        cv.text(r, 19, _hex_of(slot), S["faint"])
        cv.text(r, 35, str(SLOTS[slot][0]), S["faint"])
        cv.text(r, 47, str(SLOTS[slot][2]), S["faint"])
        # NO_COLOR column: style=None => no SGR at all (tokens.md §3.5).
        cv.text(r, 54, NOCOLOR[slot], None)
    cv.text(14, 3, "ok/err/warn 在 16 色档取亮色（92/91/93）：最近邻会把它们与 fg 撞成同一格。", S["faint"])

    # ── B · the same screen, coloured vs colourless ─────────────────────────
    # Everything at col >= NOCOLOR_COL is emitted with style=None so the
    # `.ans` carries no SGR for that region — this is the demo #25 asserts on.
    cv.text(16, 2, "B · 无色降级对照（左：256 色；右：NO_COLOR）", S["title"])
    cv.text(17, 3, "有颜色", S["dim"])
    cv.text(17, NOCOLOR_COL, "无色：语义改由字形 + 缩进 + 列位承担", None)
    exhibit = [
        ("ok", "✓", "Java 21  已检测"),
        ("err", "✗", "Java 25  未检测到"),
        ("warn", "!", "缺 Java 25：启用 foojay 自动下载？"),
        ("accent", "❯", "1.21.11   当前项"),
    ]
    for i, (slot, mark, text) in enumerate(exhibit):
        r = 18 + i
        cv.text(r, 3, mark, S[slot])
        cv.text(r, 5, text, S["fg"])
        cv.text(r, NOCOLOR_COL, mark, None)    # no colour: the glyph keeps the meaning
        cv.text(r, NOCOLOR_COL + 2, text, None)
    # dim/faint: colour is gone, so indentation and `·` take the axis over.
    # Left: dim label at col 3, faint value at col 12 (colour distinguishes).
    # Right: both are the default foreground; the value is now indented one
    # level (2 cells) and prefixed with `·` (tokens.md §3.5).
    cv.text(22, 3, "平台", S["dim"])
    cv.text(22, 12, "· paper-plugin.yml", S["faint"])
    cv.text(22, NOCOLOR_COL, "平台", None)
    cv.text(22, NOCOLOR_COL + 9, "· paper-plugin.yml", None)
    cv.text(23, 3, "↑ dim / faint 靠颜色分开", S["faint"])
    cv.text(23, NOCOLOR_COL, "↑ 同色：靠 +1 级缩进与 · 前缀分开", None)

    # ── C · glyphs and ASCII fallbacks ──────────────────────────────────────
    cv.text(25, 2, "C · 字形与 ASCII 回退（无色只去颜色，ASCII 档换字形）", S["title"])
    for c in (3, 52):
        cv.text(26, c, pad("token", 13), S["dim"])
        cv.text(26, c + 13, pad("字形", 6), S["dim"])
        cv.text(26, c + 19, pad("ASCII", 6), S["dim"])
        cv.text(26, c + 25, pad("宽", 3), S["dim"])
        cv.text(26, c + 28, "用途", S["dim"])
    for i, key in enumerate(GLYPH_ORDER):
        col = 3 if i < 10 else 52
        r = 27 + (i % 10)
        cv.text(r, col, pad(key, 13), S["dim"])
        cv.text(r, col + 13, pad(GLYPH[key], 6), S["fg"])
        cv.text(r, col + 19, pad(GLYPH_ASCII[key], 6), S["accent"])
        cv.text(r, col + 25, pad(str(dwidth(GLYPH[key])), 3), S["faint"])
        cv.text(r, col + 28, GLYPH_NOTE[key], S["faint"])

    # ── D · spacing ─────────────────────────────────────────────────────────
    cv.text(38, 2, "D · 间距（单位：终端单元格；与 direction.md §4 的 100 列分栏一致）", S["title"])
    for i, (name, val, note) in enumerate(SPACING):
        r = 39 + i
        cv.text(r, 3, pad(name, 18), S["dim"])
        cv.text(r, 22, pad(val, 8), S["accentb"])
        cv.text(r, 31, note, S["faint"])

    # ── E · hierarchy ───────────────────────────────────────────────────────
    cv.text(46, 2, "E · 层级（相邻两级至少一个非色值轴：字重 / 列位 / 缩进）", S["title"])
    cv.text(47, 3, pad("级", 10), S["dim"])
    cv.text(47, 14, pad("色值", 16), S["dim"])
    cv.text(47, 31, pad("字重", 9), S["dim"])
    cv.text(47, 41, "缩进 / 列位", S["dim"])
    for i, (lvl, colour, weight, indent) in enumerate(HIERARCHY):
        r = 48 + i
        cv.text(r, 3, pad(lvl, 10), S["fg"])
        cv.text(r, 14, pad(colour, 16), S["accentb"])
        cv.text(r, 31, pad(weight, 9), S["dim"])
        cv.text(r, 41, indent, S["faint"])

    # ── F · multi-frame glyphs (tokens.md §4, spinner) ──────────────────────
    # Unlike every other glyph, these are frame *sequences*: one token name
    # maps to a group.  Shown here because the width guarantee ("no column
    # shift across frames") is only visible when the frames are laid out.
    cv.text(53, 2, "F · 多帧字形：spinner 帧序列（等宽，逐帧不抖动列位）", S["title"])
    cv.text(54, 3, pad("token", 24), S["dim"])
    cv.text(54, 27, pad("帧数", 6), S["dim"])
    cv.text(54, 33, pad("逐帧宽", 8), S["dim"])
    cv.text(54, 41, "帧序列", S["dim"])
    for i, (key, note) in enumerate((
        ("spin.frames.unicode", "盲文 10 帧 @8 Hz = 1.25 s 一圈"),
        ("spin.frames.ascii", "整组回退：ASCII 档下不得出现盲文"),
    )):
        r = 55 + i
        frames = GLYPH[key]
        cv.text(r, 3, pad(key, 24), S["dim"])
        cv.text(r, 27, pad(str(len(frames)), 6), S["accentb"])
        widths = {dwidth(f) for f in frames}
        cv.text(r, 33, pad(",".join(str(w) for w in sorted(widths)), 8), S["faint"])
        rendered = " ".join(frames)
        cv.text(r, 41, rendered, S["fg"])
        cv.text(r, 41 + dwidth(rendered) + 2, note, S["faint"])
    cv.text(57, 3, "ASCII 档：spin.frames.unicode 整组替换为 spin.frames.ascii（不逐帧混用）。", S["faint"])
    cv.text(58, 3, "无色档：帧照常循环，只去 SGR（#23 的 H3「无色 ≠ 不动」）。", S["faint"])
    return cv


PROTOS = {
    "rec1": (rec1, "REC ① 工程"),
    "rec2": (rec2, "REC ② 构建"),
    "rec3": (rec3, "REC ③ 目标服务端"),
    "rec4": (rec4, "REC ④ 附加"),
    "p1": (p1, "P1 · 克制专业（Restrained IDE）"),
    "p2": (p2, "P2 · 分步导航（Step Rail）"),
    "p3": (p3, "P3 · 安静日志（Quiet Log）"),
    "p4": (p4, "P4 · 面板质感（Panel + Live Preview）"),
    "shell": (shell, "SHELL · 长任务 / 错误 / 完成（P1 语言）"),
    "tokens": (tokens, "TOKENS · token 表（#22：三档色值 / 字形 / 无色降级）"),
}


def render_one(key):
    fn, label = PROTOS[key]
    cv = fn()
    ansi = cv.to_ansi()
    ans_path = os.path.join(HERE, key + ".ans")
    png_path = os.path.join(HERE, key + ".png")
    with open(ans_path, "w") as f:
        f.write(ansi)
    subprocess.run(
        [sys.executable, ANSHOT, "--in", ans_path, "--out", png_path,
         "--cols", str(cv.cols), "--rows", str(cv.rows), "--trim"],
        check=True, stdout=subprocess.DEVNULL,
    )
    # ansishot writes a sibling .svg; the .ans is the source of truth here.
    svg = png_path + ".svg"
    if os.path.exists(svg):
        os.remove(svg)
    print("  %-4s %s -> %s" % (key, label, os.path.basename(png_path)))


def main():
    keys = sys.argv[1:] or list(PROTOS)
    for k in keys:
        render_one(k)


if __name__ == "__main__":
    main()
