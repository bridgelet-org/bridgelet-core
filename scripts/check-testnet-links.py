#!/usr/bin/env python3
"""Check relative Markdown links and backticked repo paths under testnet/.

Run from the repository root:  python3 scripts/check-testnet-links.py
Exit status 1 if any relative link or anchor fails to resolve.
Backticked paths that do not exist are reported as warnings only, because some
documents deliberately name not-yet-written targets.
"""
import glob
import os
import re
import sys


def slug(heading: str) -> str:
    s = heading.strip().lower()
    s = re.sub(r"[`*_]", "", s)
    s = re.sub(r"[^\w\- ]", "", s)
    return s.replace(" ", "-")


def anchors(path: str) -> set:
    try:
        text = open(path, encoding="utf-8").read()
    except OSError:
        return set()
    return {slug(m.group(1)) for m in re.finditer(r"^#{1,6}\s+(.*)$", text, re.M)}


def main() -> int:
    files = sorted(glob.glob("testnet/**/*.md", recursive=True))
    broken, warnings, total = [], [], 0
    for f in files:
        text = open(f, encoding="utf-8").read()
        prose = re.sub(r"```.*?```", "", text, flags=re.S)
        for m in re.finditer(r"\[[^\]]*\]\(([^)\s]+)\)", prose):
            link = m.group(1)
            if re.match(r"(https?:|mailto:)", link):
                continue
            total += 1
            path, _, frag = link.partition("#")
            target = f if path == "" else os.path.normpath(os.path.join(os.path.dirname(f), path))
            if not os.path.exists(target):
                broken.append((f, link, "missing file"))
            elif frag and target.endswith(".md") and frag.lower() not in anchors(target):
                broken.append((f, link, "missing anchor"))
        for m in re.finditer(
            r"`((?:testnet|docs|contracts|scripts|deployments)/[A-Za-z0-9_./-]+\.(?:md|json|sh|rs|yml|toml|py))`",
            text,
        ):
            if not os.path.exists(m.group(1)):
                warnings.append((f, m.group(1)))
    print(f"{len(files)} files, {total} relative links, {len(broken)} broken, {len(warnings)} missing backticked paths")
    for b in broken:
        print("BROKEN ", *b)
    for w in warnings:
        print("MISSING", *w)
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
