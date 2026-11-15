#!/usr/bin/env python3
"""What must not be in the public copy (docs/releasing.md, "The public copy").

  scripts/public-check.py FOLDER [PATH...]           every finding as
                                                       file:line: kind: text;
                                                       exit 1 if there is one
  scripts/public-check.py --report FOLDER [PATH...]  counts per kind and per
                                                       file; never fails

FOLDER is the tree to check: this repository (it then checks what the export
would publish: scripts/public/exclude.txt's left-out paths are skipped, a
swapped file is checked in place of the one it replaces) or an exported
copy. PATHs limit the check to those files or folders, relative to FOLDER
(`scripts/public-check.py --report . specs` checks the specs alone). The
files are git's (tracked and untracked, not ignored) when FOLDER is a git
repository, else every file but .git/ and target/.

The kinds:

  game-file   a file of a type the original game ships
  picture     any picture (a picture of the game shows the original's art)
  large       any file over 1 MB
  address     an address in the original program (@0x5e06b0, 0x5e06b0)
  offset      a struct offset (+0x38)
  event       the original program's game event names (GE_...) and numbers
              (0x20001 .. 0x20030): the specs and the code name the commands
              in our own words (`place-flag`, `extend-road`, ...)
  vtable      a virtual table
  decompiler  a decompiler's or C++ name (FUN_..., DAT_..., undefined4,
              this->, Class::methodName, a mangled name)
  tool        the investigation's tools and private paths; the words for
              its methods (decompiled, disassembly) in specs/ only (the
              README says no decompiled code is here)
  local       a local user's path (/home/NAME, C:\\Users\\NAME,
              /mnt/c/Users/NAME) or an e-mail address
  private     the private folders of the PC (the tracker, the recording
              flag's folder, the busy flag)
  workflow    a workflow that can run on a self-hosted runner, reads a
              repository variable or a secret, or has no read-only token
  repository  a repository of the maintainer's other than the public one
              (private: a stranger gets a 404): a GitHub address of it, or
              the working repository's name with its owner (also in the
              self-hosted runner's service name)
  process     the maintainer's working process, which a stranger cannot
              follow: run names (impl-, rel-, inv-, next- and a date),
              "handoff", "planning chat", "plan step" and "plan 7.11",
              finding ids of play tests (F13 and up, "finding F7"; F1 to
              F12 are keys, F32 and F64 types), the maintainer's first name
              (the owner name in addresses is another word) and the name of
              his unpublished project built on these crates
  link        a Markdown link to a file the public copy leaves out
              (scripts/public/exclude.txt), to a file or folder that is not
              in the tree, or out of the tree

The allowed exceptions are scripts/public/allow.txt, one per line with its
reason (`kind | path | pattern | reason`; kind and path are globs, the
pattern a regular expression searched in the matched text).
"""

import fnmatch
import os
import posixpath
import re
import subprocess
import sys
import urllib.parse
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
MAX_SIZE = 1_000_000
# The public repository; the owner's other repositories are private.
OWNER, PUBLIC, WORKING = "Jensenzo", "OpenS2X", "s2-reborn"

# The types the original's install holds (specs/INVENTORY.md's kinds) and
# the executable and library types.
GAME_TYPES = {
    "lua", "lst", "bbm", "lbm", "pal", "idx", "wld", "swd", "bob", "ogg",
    "wav", "mp3", "bik", "dds", "gr2", "s2m", "rc00", "sav", "fnt", "bin",
    "exe", "dll", "fmod", "fev", "fsb", "cfg", "ini", "xml", "loc", "lng",
    "ger", "eng", "dat", "ani", "cur",
}
PICTURE_TYPES = {
    "png", "jpg", "jpeg", "gif", "bmp", "tga", "tif", "tiff", "webp", "ico",
    "psd", "svg", "exr", "hdr", "ktx", "ktx2",
}

I = re.IGNORECASE
# kind, scope, pattern. Scope: "" every text file, "specs" specs/ only,
# "workflow" workflow files only.
PATTERNS = [
    ("address", "", re.compile(r"@0x[0-9a-f]{3,}", I)),
    ("address", "", re.compile(r"(?<!@)\b0x(?:00)?[4-7][0-9a-f]{5}\b", I)),
    ("offset", "", re.compile(r"\+0x[0-9a-f]{1,4}\b", I)),
    # The original's event names (case-sensitive; the lookbehind keeps
    # PAGE_SIZE, MESSAGE_TICKS and the like out) and numbers (0x20000, a
    # rule's constant in GFX-GR2-2.6, stays allowed).
    ("event", "", re.compile(r"(?<![A-Za-z0-9_])GE_(?:[A-Z][a-z]|\b)")),
    ("event", "", re.compile(r"\b0x200(?!00\b)(?:[0-2][0-9a-f]|30)\b", I)),
    ("vtable", "", re.compile(r"\bv(?:f)?tables?\b|\bvtbl\b|\bvptr\b", I)),
    ("decompiler", "", re.compile(
        r"\b(?:FUN|DAT|LAB|PTR|SUB|switchD|caseD|thunk_FUN)_[0-9a-f]{4,}\b"
        r"|\bsub_[0-9a-f]{4,}\b|\bundefined[1248]\b|\bthis->"
        r"|\b__(?:thiscall|fastcall|stdcall|cdecl)\b")),
    # A C++ method (Class::camelCase): a Rust path's items are snake_case.
    ("decompiler", "", re.compile(r"\b[A-Z][A-Za-z0-9]+::[a-z]+[A-Z][A-Za-z0-9]*\b")),
    ("decompiler", "", re.compile(r"\?\?[0-9?]?[A-Za-z_]\w*@|\?[A-Za-z_]\w*@\w*@@")),
    ("tool", "", re.compile(
        r"\bREA\b|\bIDA\b|\bGhidra\b|\bobjdump\b|\bx(?:32|64)dbg\b|\bWinDbg\b"
        r"|\bCheat ?Engine\b|\bFrida\b|\bReClass\w*"
        r"|~/s2/re\b|\bs2/re/|\bre/evidence\b", I)),
    ("tool", "specs", re.compile(r"\bdisassembl\w*|\bdecompil\w*", I)),
    ("local", "", re.compile(r"/home/(?!runner\b|user\b)[a-z_][a-z0-9_-]*", I)),
    ("local", "", re.compile(
        r"\b[a-z]:(?:\\\\?|/)Users(?:\\\\?|/)(?![%$<{(*]|Public\b|Default\b)[^\\/\s%$<{`'\")]+", I)),
    ("local", "", re.compile(r"/mnt/[a-z]/Users/(?![%$<{(*]|Public\b)[^/\s`'\"$)]+", I)),
    ("local", "", re.compile(r"[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}\b", I)),
    ("private", "", re.compile(r"\bs2-tracker\b|\bs2trace[\\/]|\bpc-busy\.lock\b")),
    # A GitHub address of another repository of the owner's (`.git` and
    # what follows the name left out), and the working repository by owner
    # and name in any form.
    ("repository", "", re.compile(
        r"github\.com[/:]" + OWNER + r"/(?!" + PUBLIC + r"(?:\.git)?(?![\w-]|\.\w))"
        r"[\w-]+(?:\.(?!git\b)[\w-]+)*"
        r"|\b" + OWNER + r"[/-]" + WORKING + r"\b", I)),
    # The working process (run names, the planning's words, finding ids,
    # the maintainer's first name, the unpublished project).
    ("process", "", re.compile(
        r"\b(?:impl|rel|inv|next)-20\d\d-\d\d(?:-\d\d)?[\w-]*"
        r"|\bhand-?offs?\b|\bplanning chat\b|\bplan steps?\b|\bplan \d+\.\d+\w*"
        r"|\bfindings? F\d+\b|\bs2-next\b", I)),
    ("process", "", re.compile(r"\bF(?!32\b|64\b)(?:1[3-9]|[2-9]\d|\d{3})\b|\bJens\b")),
    ("workflow", "workflow", re.compile(
        r"self-hosted|\bCI_RUNNER\b|\bS2_FIDELITY_GAME_DIR\b|runs-on:\s*\$\{\{"
        r"|\bpull_request_target\b|\bsecrets\.|\bvars\.|:\s*write\b")),
]


def read_list(path):
    """The lines of a list file without comments, split at ' | '."""
    if not os.path.isfile(path):
        return []
    rows = []
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            rows.append((n, [p.strip() for p in line.split(" | ")]))
    return rows


def exclusions(folder):
    """(left-out paths, {target: source}) of the folder's exclude.txt."""
    left, swapped = [], {}
    for n, parts in read_list(os.path.join(folder, "scripts", "public", "exclude.txt")):
        if len(parts) < 2 or not parts[-1]:
            sys.exit(f"scripts/public/exclude.txt:{n}: a path and a reason, separated by ' | '")
        spec = parts[0]
        if " <- " in spec:
            target, source = (s.strip() for s in spec.split(" <- "))
            swapped[target] = source
        else:
            left.append(spec)
    return left, swapped


def allowed(folder):
    rows = []
    for n, parts in read_list(os.path.join(folder, "scripts", "public", "allow.txt")):
        if len(parts) != 4 or not parts[3]:
            sys.exit(f"scripts/public/allow.txt:{n}: kind | path | pattern | reason")
        rows.append((parts[0], parts[1], re.compile(parts[2]), n))
    return rows


def files(folder):
    if os.path.isdir(os.path.join(folder, ".git")):
        out = subprocess.run(
            ["git", "-C", folder, "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            check=True, capture_output=True).stdout.decode()
        names = sorted({p for p in out.split("\0") if p})
        return [p for p in names if os.path.isfile(os.path.join(folder, p))]
    names = []
    for root, dirs, fs in os.walk(folder):
        dirs[:] = [d for d in dirs if not (root == folder and d in (".git", "target"))]
        for f in fs:
            names.append(os.path.relpath(os.path.join(root, f), folder).replace(os.sep, "/"))
    return sorted(names)


def under(path, prefix):
    prefix = prefix.rstrip("/")
    return path == prefix or path.startswith(prefix + "/")


MD_INLINE = re.compile(r"\]\(\s*(<[^>]*>|[^)\s]+)")
MD_REFERENCE = re.compile(r"^ {0,3}\[[^\]]+\]:\s*(<[^>]*>|\S+)")
MD_CODE_SPAN = re.compile(r"(`+).*?\1")
MD_FENCE = re.compile(r"^ {0,3}(```|~~~)")


def md_links(text):
    """(line, target) of every link of a Markdown text: inline links and
    pictures, and reference definitions; not in code spans or blocks."""
    fence = None
    for n, line in enumerate(text.splitlines(), 1):
        m = MD_FENCE.match(line)
        if m:
            fence = None if fence == m.group(1) else (fence or m.group(1))
            continue
        if fence:
            continue
        line = MD_CODE_SPAN.sub("", line)
        for m in list(MD_REFERENCE.finditer(line)) + list(MD_INLINE.finditer(line)):
            yield n, m.group(1)


def link_finding(path, target, published, dirs, left, sources):
    """Why a link of `path` to `target` leads nowhere in the public copy,
    or None."""
    target = target.strip("<>")
    if not target or target.startswith(("#", "//")) or re.match(r"[A-Za-z][A-Za-z0-9+.-]*:", target):
        return None
    rel = urllib.parse.unquote(re.split(r"[#?]", target, maxsplit=1)[0])
    if not rel:
        return None
    if rel.startswith("/"):
        full = posixpath.normpath(rel.lstrip("/"))
    else:
        full = posixpath.normpath(posixpath.join(posixpath.dirname(path), rel))
    if full == ".." or full.startswith("../"):
        return "outside the tree"
    if any(under(full, p) for p in left) or full in sources:
        return "left out of the public copy"
    if full != "." and full not in published and full not in dirs:
        return "not in the tree"
    return None


def check(folder, only):
    left, swapped = exclusions(folder)
    allow = allowed(folder)
    used = Counter()
    findings = []  # (file, line, kind, text)

    def keep(kind, path, text):
        for k, glob, pat, n in allow:
            if fnmatch.fnmatch(kind, k) and fnmatch.fnmatch(path, glob) and pat.search(text):
                used[n] += 1
                return False
        return True

    sources = {s for t, s in swapped.items() if os.path.isfile(os.path.join(folder, s))}
    every = files(folder)
    # What the public copy holds: a link must lead into it.
    published = {p for p in every if not any(under(p, q) for q in left)
                 and p not in swapped.values()} | set(swapped)
    dirs = {posixpath.dirname(p) for p in published}
    for d in list(dirs):
        while d:
            d = posixpath.dirname(d)
            dirs.add(d)
    for path in every:
        if any(under(path, p) for p in left):
            continue
        # A swapped file: its source is checked in its place.
        if path in swapped and swapped[path] in sources:
            continue
        if only and not any(under(path, p) for p in only):
            continue
        full = os.path.join(folder, path)
        ext = path.rsplit(".", 1)[-1].lower() if "." in os.path.basename(path) else ""
        if ext in GAME_TYPES and keep("game-file", path, path):
            findings.append((path, 0, "game-file", f".{ext}"))
        if ext in PICTURE_TYPES and keep("picture", path, path):
            findings.append((path, 0, "picture", f".{ext}"))
        size = os.path.getsize(full)
        if size > MAX_SIZE and keep("large", path, path):
            findings.append((path, 0, "large", f"{size} bytes"))
        with open(full, "rb") as f:
            data = f.read()
        if b"\0" in data[:8192]:
            continue
        text = data.decode("utf-8", errors="replace")
        workflow = path in sources or (
            under(path, ".github/workflows") and ext in ("yml", "yaml"))
        in_specs = under(path, "specs")
        for n, line in enumerate(text.splitlines(), 1):
            for kind, scope, pat in PATTERNS:
                if scope == "specs" and not in_specs:
                    continue
                if scope == "workflow" and not workflow:
                    continue
                for m in pat.finditer(line):
                    if keep(kind, path, m.group(0)):
                        findings.append((path, n, kind, m.group(0)))
        if ext == "md":
            for n, target in md_links(text):
                why = link_finding(path, target, published, dirs, left, set(swapped.values()))
                if why and keep("link", path, target):
                    findings.append((path, n, "link", f"{target} ({why})"))
        if workflow and not re.search(r"(?m)^permissions:\s*\n\s+contents:\s*read\s*$", text):
            if keep("workflow", path, "permissions"):
                findings.append((path, 1, "workflow", "no top-level `permissions: contents: read`"))
    return findings, used, allow


def main():
    args = sys.argv[1:]
    report = "--report" in args
    args = [a for a in args if a != "--report"]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__.strip())
        return 0 if args else 2
    folder, only = args[0], [p.strip("/") for p in args[1:]]
    if not os.path.isdir(folder):
        sys.exit(f"no folder {folder}")
    findings, used, allow = check(folder, only)
    if report:
        kinds = Counter(k for _, _, k, _ in findings)
        per_file = defaultdict(Counter)
        for f, _, k, _ in findings:
            per_file[f][k] += 1
        print(f"{len(findings)} findings in {len(per_file)} files")
        for k, c in kinds.most_common():
            print(f"  {k:<11} {c}")
        if per_file:
            print("per file:")
            for f in sorted(per_file, key=lambda f: (-sum(per_file[f].values()), f)):
                counts = ", ".join(f"{k} {c}" for k, c in per_file[f].most_common())
                print(f"  {sum(per_file[f].values()):>5}  {f}  ({counts})")
        print(f"allowed exceptions used: {sum(used.values())} matches of "
              f"{len(used)} of the {len(allow)} entries in scripts/public/allow.txt")
        return 0
    for f, n, k, t in findings:
        print(f"{f}:{n}: {k}: {t}")
    if findings:
        print(f"{len(findings)} findings (scripts/public-check.py --report {folder} for counts)",
              file=sys.stderr)
        return 1
    print(f"public check: nothing found in {folder}"
          + (f" ({', '.join(only)})" if only else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
