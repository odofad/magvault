#!/usr/bin/env python3
"""Drive the real `magvault` TUI in a pseudo-terminal (a bot at the keyboard).

    tui_pty.py SCRIPT -- magvault --config DIR ui [...]

SCRIPT lines (blank lines and # comments ignored):
    send TEXT       type TEXT; escapes: \\r Enter, \\t Tab, \\e Esc, \\x7f Backspace,
                    \\eOB / \\e[B arrows ("down"/"up"/"left"/"right"/"f1" send
                    the whole key sequence at once)
    down | up       arrow key
    wait TEXT       wait (up to 60 s) until TEXT appears on the screen
    seek TEXT       press Down until the selected row (▶) contains TEXT
    sleep SECONDS
    exit CODE       wait for the program to end with CODE

The terminal is 150x42; a small VT100 emulator keeps the screen, and `wait` searches the
current screen. On failure the screen is printed. Exit status 0 when every step passed.
"""
import fcntl
import os
import pty
import re
import select
import signal
import struct
import sys
import termios
import time

W, H = 150, 42
CSI = re.compile(r"\x1b\[([0-?]*)[ -/]*([@-~])|\x1b\][^\x07]*\x07|\x1b[()][0-9A-Za-z]|\x1b[=>78]")


class Screen:
    """Just enough of a VT100 for ratatui/crossterm output and plain printed lines."""

    def __init__(self):
        self.rows = [[" "] * W for _ in range(H)]
        self.x = self.y = 0
        self.pending = ""

    def text(self):
        return "\n".join("".join(r) for r in self.rows)

    def clear(self):
        self.rows = [[" "] * W for _ in range(H)]

    def newline(self):
        self.y += 1
        if self.y >= H:
            self.rows.pop(0)
            self.rows.append([" "] * W)
            self.y = H - 1

    def feed(self, data):
        s = self.pending + data
        self.pending = ""
        i = 0
        while i < len(s):
            c = s[i]
            if c == "\x1b":
                m = CSI.match(s, i)
                if not m:
                    self.pending = s[i:]  # incomplete sequence: keep for the next read
                    return
                if m.group(2):
                    self.csi(m.group(1), m.group(2))
                i = m.end()
                continue
            if c == "\r":
                self.x = 0
            elif c == "\n":
                self.newline()
            elif c == "\b":
                self.x = max(0, self.x - 1)
            elif c >= " ":
                if self.x >= W:
                    self.x = 0
                    self.newline()
                self.rows[self.y][self.x] = c
                self.x += 1
            i += 1

    def csi(self, params, op):
        if params.startswith("?"):
            if params in ("?1049h", "?1049l", "?47h", "?47l") or params[1:] in ("1049", "47"):
                self.clear()
            return
        nums = [int(p) if p.isdigit() else 0 for p in params.split(";")] if params else []
        n = (nums[0] or 1) if nums else 1
        if op in "Hf":
            r = (nums[0] if len(nums) > 0 and nums[0] else 1) - 1
            c = (nums[1] if len(nums) > 1 and nums[1] else 1) - 1
            self.y, self.x = min(r, H - 1), min(c, W - 1)
        elif op == "A":
            self.y = max(0, self.y - n)
        elif op == "B":
            self.y = min(H - 1, self.y + n)
        elif op == "C":
            self.x = min(W - 1, self.x + n)
        elif op == "D":
            self.x = max(0, self.x - n)
        elif op == "G":
            self.x = min(W - 1, n - 1)
        elif op == "J":
            mode = nums[0] if nums else 0
            if mode == 2 or mode == 3:
                self.clear()
            elif mode == 0:
                self.rows[self.y][self.x :] = [" "] * (W - self.x)
                for r in range(self.y + 1, H):
                    self.rows[r] = [" "] * W
        elif op == "K":
            mode = nums[0] if nums else 0
            if mode == 0:
                self.rows[self.y][self.x :] = [" "] * (W - self.x)
            elif mode == 2:
                self.rows[self.y] = [" "] * W


def main():
    if "--" not in sys.argv:
        print(__doc__)
        return 2
    i = sys.argv.index("--")
    script, cmd = sys.argv[1:i], sys.argv[i + 1 :]
    steps = [l.rstrip("\n") for l in open(script[0]) if l.strip() and not l.lstrip().startswith("#")]

    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "xterm-256color"
        os.execvp(cmd[0], cmd)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", H, W, 0, 0))
    os.kill(pid, signal.SIGWINCH)
    scr = Screen()
    status = None

    def pump(timeout):
        nonlocal status
        r, _, _ = select.select([fd], [], [], timeout)
        if r:
            try:
                data = os.read(fd, 65536)
            except OSError:
                data = b""
            scr.feed(data.decode("utf-8", "replace"))
        if status is None:
            p, st = os.waitpid(pid, os.WNOHANG)
            if p:
                status = os.waitstatus_to_exitcode(st)

    def fail(msg):
        print(f"tui_pty: FAIL {msg}\n----- screen -----\n{scr.text()}\n-----")
        if status is None:
            os.kill(pid, signal.SIGKILL)
        sys.exit(1)

    keys = {"down": "\x1b[B", "up": "\x1b[A", "right": "\x1b[C", "left": "\x1b[D", "f1": "\x1bOP"}
    for step in steps:
        word, _, arg = step.strip().partition(" ")
        if word in keys:
            os.write(fd, keys[word].encode())
            time.sleep(0.15)
        elif word == "send":
            text = arg.replace("\\e", "\x1b").encode().decode("unicode_escape")
            for chunk in re.split("(\x1b)", text):
                if chunk:
                    os.write(fd, chunk.encode())
                    time.sleep(0.15 if chunk == "\x1b" else 0.05)
        elif word == "wait":
            end = time.time() + 60
            while arg not in scr.text():
                if time.time() > end:
                    fail(f"timed out waiting for {arg!r}")
                if status is not None:
                    fail(f"program ended ({status}) before {arg!r}")
                pump(0.2)
        elif word == "seek":
            # Press Down until the selected row (▶) shows TEXT.
            for _ in range(25):
                pump(0.3)
                if any("▶" in l and arg in l for l in scr.text().split("\n")):
                    break
                os.write(fd, keys["down"].encode())
            else:
                fail(f"no selectable row with {arg!r}")
        elif word == "sleep":
            end = time.time() + float(arg)
            while time.time() < end:
                pump(0.1)
        elif word == "exit":
            end = time.time() + 30
            while status is None:
                if time.time() > end:
                    fail("program did not exit")
                pump(0.2)
            if status != int(arg):
                fail(f"exit code {status}, expected {arg}")
        else:
            fail(f"bad script line: {step}")
    if os.environ.get("TUI_PTY_DUMP"):
        print(scr.text())
    return 0


if __name__ == "__main__":
    sys.exit(main())
