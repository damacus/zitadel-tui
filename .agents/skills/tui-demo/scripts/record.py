#!/usr/bin/env python3
"""Record scripted terminal input and real terminal output as asciicast v2."""
import argparse
import codecs
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import termios
import time


def record(args):
    steps = json.loads(args.steps.read_text())
    pid, fd = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", args.rows, args.cols, 0, 0))
        os.environ["TERM"] = "xterm-256color"
        os.environ.pop("NO_COLOR", None)
        os.execvp(args.command[0], args.command)

    decoder = codecs.getincrementaldecoder("utf-8")()
    events = []
    clock = time.monotonic()

    def read_for(seconds):
        deadline = time.monotonic() + seconds
        chunks = []
        while (remaining := deadline - time.monotonic()) > 0:
            if not select.select([fd], [], [], remaining)[0]:
                break
            try:
                data = os.read(fd, 65536)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
                return "".join(chunks), True
            if not data:
                return "".join(chunks), True
            if b"\x1b[6n" in data:
                os.write(fd, b"\x1b[1;1R")
            output = decoder.decode(data)
            events.append([round(time.monotonic() - clock, 4), "o", output])
            chunks.append(output)
        return "".join(chunks), False

    reaped = False
    try:
        first, closed = read_for(args.startup)
        if closed:
            raise RuntimeError("Terminal closed before the walkthrough began")
        if args.ready not in first:
            raise RuntimeError(f"Starting screen did not contain {args.ready!r}")
        events[:] = [[0, "o", first]]
        clock = time.monotonic() - args.opening
        for index, step in enumerate(steps, 1):
            os.write(fd, step["keys"].encode())
            output, closed = read_for(step["hold"])
            if "expect" in step and step["expect"] not in output:
                raise RuntimeError(f"Step {index} did not display {step['expect']!r}")
            if closed:
                if index < len(steps):
                    raise RuntimeError(f"Terminal closed at step {index} of {len(steps)}")
                elapsed = time.monotonic()
                while time.monotonic() - elapsed < 0.5:
                    finished, status = os.waitpid(pid, os.WNOHANG)
                    if finished:
                        reaped = True
                        if os.waitstatus_to_exitcode(status) != 0:
                            raise RuntimeError("Application exited unsuccessfully on the final step")
                        break
                    time.sleep(0.01)
                if not reaped:
                    raise RuntimeError("Terminal closed without a normal application exit")
        # Reset rendition without changing any visible cells. The timestamp preserves idle time.
        events.append([round(time.monotonic() - clock, 4), "o", "[0m"])
        header = {"version": 2, "width": args.cols, "height": args.rows}
        args.output.write_text("".join(json.dumps(row) + "\n" for row in [header, *events]))
    finally:
        os.close(fd)
        if not reaped:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            os.waitpid(pid, 0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cols", type=int, default=100)
    parser.add_argument("--rows", type=int, default=26)
    parser.add_argument("--startup", type=float, default=1.5)
    parser.add_argument("--opening", type=float, default=2)
    parser.add_argument("--ready", required=True)
    parser.add_argument("--steps", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command.pop(0)
    if not args.command:
        parser.error("supply a command after --")
    record(args)
