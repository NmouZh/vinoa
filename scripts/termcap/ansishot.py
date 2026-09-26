#!/usr/bin/env python3
"""Throwaway prototype: render raw terminal bytes to a PNG screenshot.

Not a real terminal emulator -- it implements the subset of VT that the vinoa
TUI actually emits (SGR, cursor movement, erase, alt-screen) on a fixed cell
grid. The point is that a human (or a vision model) can *look* at the interface
from an environment whose own TERM is dumb.

Usage:
  python3 ansishot.py --in raw.bin --out shot.png [--cols 100] [--rows 40]
"""
import argparse
import re
import subprocess
import sys
import unicodedata

# xterm-ish default palette for SGR 0-15.
PALETTE = [
    "#000000", "#cd0000", "#00cd00", "#cdcd00", "#0000ee", "#cd00cd", "#00cdcd", "#e5e5e5",
    "#7f7f7f", "#ff0000", "#00ff00", "#ffff00", "#5c5cff", "#ff00ff", "#00ffff", "#ffffff",
]
DEFAULT_FG = "#cdd6f4"
DEFAULT_BG = "#1e1e2e"


def cell_width(ch):
    if unicodedata.combining(ch):
        return 0
    if unicodedata.east_asian_width(ch) in ("W", "F"):
        return 2
    return 1


class Cell:
    __slots__ = ("ch", "fg", "bg", "bold", "dim", "italic", "underline", "reverse", "cont")

    def __init__(self, ch=" ", fg=DEFAULT_FG, bg=DEFAULT_BG, **kw):
        self.ch = ch
        self.fg = fg
        self.bg = bg
        self.bold = kw.get("bold", False)
        self.dim = kw.get("dim", False)
        self.italic = kw.get("italic", False)
        self.underline = kw.get("underline", False)
        self.reverse = kw.get("reverse", False)
        self.cont = kw.get("cont", False)

    def style_key(self):
        return (self.fg, self.bg, self.bold, self.dim, self.italic, self.underline, self.reverse)


class Terminal:
    def __init__(self, cols, rows):
        self.cols, self.rows = cols, rows
        self.grid = [[Cell() for _ in range(cols)] for _ in range(rows)]
        self.r = self.c = 0
        self.fg, self.bg = DEFAULT_FG, DEFAULT_BG
        self.bold = self.dim = self.italic = self.underline = self.reverse = False

    # -- grid helpers -------------------------------------------------------
    def reset_style(self):
        self.fg, self.bg = DEFAULT_FG, DEFAULT_BG
        self.bold = self.dim = self.italic = self.underline = self.reverse = False

    def scroll(self):
        self.grid.pop(0)
        self.grid.append([Cell() for _ in range(self.cols)])

    def newline(self):
        self.r += 1
        if self.r >= self.rows:
            self.r = self.rows - 1
            self.scroll()

    def _fresh(self):
        return Cell(fg=self.fg, bg=self.bg, bold=self.bold, dim=self.dim,
                    italic=self.italic, underline=self.underline, reverse=self.reverse)

    def put(self, ch):
        w = cell_width(ch)
        if w == 0:
            # Combining mark: attach to the previous cell.
            if self.c > 0:
                self.grid[self.r][self.c - 1].ch += ch
            return
        if self.c + w > self.cols:
            self.c = 0
            self.newline()
        self.grid[self.r][self.c] = self._fresh()
        self.grid[self.r][self.c].ch = ch
        if w == 2:
            cont = self._fresh()
            cont.ch = ""
            cont.cont = True
            if self.c + 1 < self.cols:
                self.grid[self.r][self.c + 1] = cont
        self.c += w

    def erase_line(self, mode):
        if mode == 2:
            span = range(self.cols)
        elif mode == 0:
            span = range(self.c, self.cols)
        else:
            span = range(0, self.c + 1)
        for i in span:
            self.grid[self.r][i] = Cell(fg=self.fg, bg=self.bg)

    def erase_display(self, mode):
        if mode == 2:
            self.grid = [[Cell() for _ in range(self.cols)] for _ in range(self.rows)]

    # -- SGR ---------------------------------------------------------------
    def sgr(self, params):
        if not params:
            params = [0]
        i = 0
        while i < len(params):
            p = params[i]
            if p == 0:
                self.reset_style()
            elif p == 1:
                self.bold = True
            elif p == 2:
                self.dim = True
            elif p == 3:
                self.italic = True
            elif p == 4:
                self.underline = True
            elif p == 7:
                self.reverse = True
            elif p in (21, 22):
                self.bold = self.dim = False
            elif p == 23:
                self.italic = False
            elif p == 24:
                self.underline = False
            elif p == 27:
                self.reverse = False
            elif 30 <= p <= 37:
                self.fg = PALETTE[p - 30]
            elif p == 39:
                self.fg = DEFAULT_FG
            elif 40 <= p <= 47:
                self.bg = PALETTE[p - 40]
            elif p == 49:
                self.bg = DEFAULT_BG
            elif 90 <= p <= 97:
                self.fg = PALETTE[p - 90 + 8]
            elif 100 <= p <= 107:
                self.bg = PALETTE[p - 100 + 8]
            elif p in (38, 48):
                target = "fg" if p == 38 else "bg"
                if i + 1 < len(params) and params[i + 1] == 5 and i + 2 < len(params):
                    setattr(self, target, xterm256(params[i + 2]))
                    i += 2
                elif i + 1 < len(params) and params[i + 1] == 2 and i + 4 < len(params):
                    r, g, b = params[i + 2], params[i + 3], params[i + 4]
                    setattr(self, target, "#%02x%02x%02x" % (r, g, b))
                    i += 4
            i += 1


def xterm256(n):
    if n < 16:
        return PALETTE[n]
    if n < 232:
        n -= 16
        r, g, b = n // 36, (n % 36) // 6, n % 6
        conv = lambda v: 0 if v == 0 else 55 + v * 40
        return "#%02x%02x%02x" % (conv(r), conv(g), conv(b))
    v = 8 + (n - 232) * 10
    return "#%02x%02x%02x" % (v, v, v)


CSI_RE = re.compile(r"\x1b\[([0-9;?]*)([a-zA-Z])")
OSC_RE = re.compile(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")


def feed(term, data):
    text = data.decode("utf-8", "replace")
    i = 0
    while i < len(text):
        ch = text[i]
        if ch == "\x1b":
            m = OSC_RE.match(text, i)
            if m:
                i = m.end()
                continue
            m = CSI_RE.match(text, i)
            if m:
                params_raw, final = m.group(1), m.group(2)
                if params_raw.startswith("?"):
                    i = m.end()  # private modes (cursor visibility, alt screen)
                    continue
                params = [int(x) for x in params_raw.split(";") if x != ""] if params_raw else []
                if final == "m":
                    term.sgr(params)
                elif final == "A":
                    term.r = max(0, term.r - max(1, params[0] if params else 1))
                elif final == "B":
                    term.r = min(term.rows - 1, term.r + max(1, params[0] if params else 1))
                elif final == "C":
                    term.c = min(term.cols - 1, term.c + max(1, params[0] if params else 1))
                elif final == "D":
                    term.c = max(0, term.c - max(1, params[0] if params else 1))
                elif final == "H" or final == "f":
                    row = (params[0] if len(params) > 0 else 1) - 1
                    col = (params[1] if len(params) > 1 else 1) - 1
                    term.r, term.c = max(0, min(term.rows - 1, row)), max(0, min(term.cols - 1, col))
                elif final == "K":
                    term.erase_line(params[0] if params else 0)
                elif final == "J":
                    term.erase_display(params[0] if params else 0)
                i = m.end()
                continue
            i += 1
            continue
        if ch == "\r":
            term.c = 0
        elif ch == "\n":
            term.newline()
        elif ch == "\b":
            term.c = max(0, term.c - 1)
        elif ch == "\t":
            term.c = min(term.cols - 1, (term.c // 8 + 1) * 8)
        elif ch >= " " or ch == "\x00":
            if ch != "\x00":
                term.put(ch)
        i += 1


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def to_svg(term, cw, chh, font, pad=12):
    w = term.cols * cw + pad * 2
    h = term.rows * chh + pad * 2
    out = [
        '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d">' % (w, h),
        '<rect width="%d" height="%d" fill="%s"/>' % (w, h, DEFAULT_BG),
        '<g font-family="%s" font-size="%.1f">' % (font, chh * 0.78),
    ]
    # Background runs.
    for r, row in enumerate(term.grid):
        c = 0
        while c < term.cols:
            bg = row[c].bg
            start = c
            while c < term.cols and row[c].bg == bg:
                c += 1
            if bg != DEFAULT_BG:
                out.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="%s"/>' % (
                    pad + start * cw, pad + r * chh, (c - start) * cw, chh, bg))
    # Glyphs, positioned per cell so the monospace grid cannot drift.
    for r, row in enumerate(term.grid):
        y = pad + r * chh + chh * 0.78
        for c, cell in enumerate(row):
            if cell.cont or not cell.ch or cell.ch == " ":
                continue
            fg, bg = cell.fg, cell.bg
            if cell.reverse:
                fg, bg = bg, fg
            weight = ' font-weight="bold"' if cell.bold else ""
            style = ' font-style="italic"' if cell.italic else ""
            deco = ' text-decoration="underline"' if cell.underline else ""
            w2 = cell_width(cell.ch) == 2
            x = pad + c * cw + (cw if w2 else 0)
            anchor = ' text-anchor="middle"' if w2 else ""
            out.append('<text x="%.1f" y="%.1f" fill="%s"%s%s%s%s>%s</text>' % (
                x, y, fg, weight, style, deco, anchor, esc(cell.ch)))
    out.append("</g></svg>")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="src", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--cols", type=int, default=100)
    ap.add_argument("--rows", type=int, default=40)
    ap.add_argument("--cell-w", type=float, default=9.0)
    ap.add_argument("--cell-h", type=float, default=19.0)
    ap.add_argument("--font", default="DejaVu Sans Mono, Noto Sans Mono CJK SC, monospace")
    ap.add_argument("--trim", action="store_true", help="drop trailing all-blank rows")
    a = ap.parse_args()

    data = open(a.src, "rb").read()
    term = Terminal(a.cols, a.rows)
    feed(term, data)

    if a.trim:
        while term.grid and all(c.ch in ("", " ") for c in term.grid[-1]):
            term.grid.pop()
        term.rows = len(term.grid)

    svg = to_svg(term, a.cell_w, a.cell_h, a.font)
    svg_path = a.out + ".svg"
    open(svg_path, "w").write(svg)
    subprocess.run(["rsvg-convert", "-o", a.out, svg_path], check=True)
    print("wrote %s (%dx%d cells)" % (a.out, term.cols, term.rows))


if __name__ == "__main__":
    main()
