#!/usr/bin/env python3
"""Run Studio's subscription-only cost-contract gate."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def main() -> int:
    repo = Path(__file__).resolve().parents[1]
    command = ["cargo", "test", "-p", "loom-core", "studio_crates_never_call_metered_apis"]
    completed = subprocess.run(command, cwd=repo)
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())
