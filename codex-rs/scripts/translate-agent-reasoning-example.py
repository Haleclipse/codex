#!/usr/bin/env python3

import json
import sys


def main() -> int:
    req = json.load(sys.stdin)

    text = req.get("text", "")
    kind = req.get("kind", "")

    # This is only an example: it does not do real translation.
    # Replace this with your own translator implementation.
    prefix = "译: " if kind == "agent_reasoning_title" else ""
    out = {"schema_version": 1, "text": f"{prefix}{text}"}

    sys.stdout.write(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

