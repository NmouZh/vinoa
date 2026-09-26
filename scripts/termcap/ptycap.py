#!/usr/bin/env python3
"""Capture a command's raw terminal output through a pty of an exact size,
optionally driving it with a scripted key schedule so interactive pages can be
walked and captured.
"""
import argparse
import fcntl
import os
import pty
import select
import struct
import sys
import termios
import threading
import time


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--cols", type=int, default=100)
    ap.add_argument("--rows", type=int, default=40)
    ap.add_argument("--term", default="xterm-256color")
    ap.add_argument("--colorterm", default="truecolor")
    ap.add_argument("--timeout", type=float, default=30.0)
    ap.add_argument("--keys", default="",
                    help="comma-separated 'delay:key' pairs, e.g. '0.6:\\r,1.2:\\r,2:n'")
    ap.add_argument("--cwd", default=None)
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    cmd = a.cmd
    if cmd and cmd[0] == "--":
        cmd = cmd[1:]

    schedule = []
    for item in [x for x in a.keys.split(",") if x.strip()]:
        delay, _, key = item.partition(":")
        schedule.append((float(delay), key.encode().decode("unicode_escape").encode()))

    pid, fd = pty.fork()
    if pid == 0:
        if a.cwd:
            os.chdir(a.cwd)
        env = dict(os.environ)
        env["TERM"] = a.term
        env["COLORTERM"] = a.colorterm
        env.pop("NO_COLOR", None)
        env["LC_ALL"] = "C.UTF-8"
        env["LANG"] = "C.UTF-8"
        os.execvpe(cmd[0], cmd, env)

    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", a.rows, a.cols, 0, 0))

    buf = bytearray()
    lock = threading.Lock()
    start = time.time()

    def sender():
        for delay, key in schedule:
            target = start + delay
            while time.time() < target:
                time.sleep(0.02)
            try:
                os.write(fd, key)
            except OSError:
                return

    th = threading.Thread(target=sender, daemon=True)
    th.start()

    deadline = start + a.timeout
    while time.time() < deadline:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            with lock:
                buf.extend(chunk)
        else:
            pid_done, _ = os.waitpid(pid, os.WNOHANG)
            if pid_done:
                try:
                    while True:
                        r2, _, _ = select.select([fd], [], [], 0.15)
                        if not r2:
                            break
                        chunk = os.read(fd, 65536)
                        if not chunk:
                            break
                        buf.extend(chunk)
                except OSError:
                    pass
                break
    try:
        os.kill(pid, 9)
    except ProcessLookupError:
        pass
    with open(a.out, "wb") as f:
        f.write(bytes(buf))
    print("captured %d bytes -> %s (%.1fs)" % (len(buf), a.out, time.time() - start))


if __name__ == "__main__":
    main()
