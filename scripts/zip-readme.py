#!/usr/bin/env python3
"""The README for the Windows download (scripts/dist.sh).

  scripts/zip-readme.py README.md > OUT

GitHub shows the README's title block, badges and pictures; a text editor
shows their HTML. This writes the README without them: the centred title
becomes a Markdown heading, a centred text block plain text (bold kept as
**), every centred block that holds pictures or badges goes, and one line in
place of the first says where the pictures are.
"""

import re
import sys

PAGE = "https://github.com/Jensenzo/OpenS2X"


def convert(text):
    out, said = [], False
    for block in re.split(r"\n\s*\n", text.strip("\n")):
        m = re.fullmatch(r"\s*<h1[^>]*>(.*?)</h1>\s*", block, re.S)
        if m:
            out.append(f"# {m.group(1).strip()}")
        elif re.match(r"\s*<p align=", block):
            if "<img" in block:
                if not said:
                    out.append(f"The pictures of this page are on {PAGE}.")
                    said = True
                continue
            plain = re.sub(r"</?p[^>]*>", "", block)
            plain = re.sub(r"<br\s*/?>", " ", plain)
            plain = re.sub(r"</?b>", "**", plain)
            plain = re.sub(r"</?i>", "*", plain)
            plain = re.sub(r"<[^>]+>", "", plain)
            out.append(re.sub(r"\s+", " ", plain).strip())
        else:
            out.append(block)
    return "\n\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(convert(open(sys.argv[1], encoding="utf-8").read()))
