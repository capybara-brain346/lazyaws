import fcntl
import json
import os
import pty
import select
import signal
import struct
import sys
import termios
import time
from pathlib import Path


binary = os.path.abspath(sys.argv[1])
config = json.loads((Path(__file__).resolve().parents[1] / "distribution/config.json").read_text())
pid, terminal = pty.fork()
if pid == 0:
    os.execv(binary, [binary, "--demo"])

fcntl.ioctl(terminal, termios.TIOCSWINSZ, struct.pack("HHHH", config["demoCheckRows"], config["demoCheckColumns"], 0, 0))
deadline = time.monotonic() + config["demoCheckTimeoutSeconds"]
sent_quit = False

while time.monotonic() < deadline:
    readable, _, _ = select.select([terminal], [], [], 0.1)
    if readable:
        try:
            output = os.read(terminal, 65536)
        except OSError:
            output = b""
        if output and not sent_quit:
            os.write(terminal, b"q")
            sent_quit = True

    finished, status = os.waitpid(pid, os.WNOHANG)
    if finished:
        if sent_quit and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0:
            sys.exit(0)
        raise SystemExit("demo did not exit cleanly after q")

os.kill(pid, signal.SIGKILL)
os.waitpid(pid, 0)
raise SystemExit("demo did not exit after q before the configured timeout")
