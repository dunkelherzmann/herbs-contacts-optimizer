#!/usr/bin/env python3
"""Notes on enhancements.md lines that a platform is done: mark-done.py "text in the line" "Mac done (0.1)"."""
import sys
needle, note = sys.argv[1], sys.argv[2]
path = "enhancements.md"
lines = open(path).read().split("\n")
hits = [i for i, l in enumerate(lines) if needle in l]
assert len(hits) == 1, f"{needle!r} matches {len(hits)} lines"
lines[hits[0]] += f" ({note})"
open(path, "w").write("\n".join(lines))
