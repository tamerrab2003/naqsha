#!/usr/bin/env python3
"""Interface strings for translation.

    tools/i18n.py list           # every English string the app shows, one JSON array (stdout)
    tools/i18n.py missing CODE   # the ones locales/CODE.json doesn't translate yet
    tools/i18n.py merge CODE F   # add the translations in F (a JSON object) to locales/CODE.json
    tools/i18n.py prune          # drop translations of strings the app no longer has

The app looks translations up by their exact English text (src/i18n.rs), so a string changed in
the code needs translating again; `missing` lists it.
"""
import json, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIT = r'"((?:[^"\\]|\\.)*)"'

# files whose string literals are mostly interface text: every literal that looks like words
UI_FILES = ["src/ui.rs", "src/app.rs", "src/canvas.rs", "src/main.rs", "src/shell.rs", "src/embed.rs",
            "src/subpage.rs", "src/live.rs",
            "pro/src/ui.rs", "pro/src/license.rs", "pro/src/catalog.rs", "pro/src/production.rs",
            "pro/src/drawset.rs", "pro/src/sheets.rs", "pro/src/schedule.rs", "pro/src/cad.rs", "pro/src/mod.rs"]


# literals that look like words but aren't shown to people
SKIP = {"Content-Type", "Accept", "NAQSHA-CLIP:", "SCHEMIX-CLIP:", "Naqsha", "Jenkinsfile", "Visio", "Pro",
        "Node-RED", "CI/CD", "Abc", "Aa", "V C T H", "-{out}", "img-{id}", "no window", "missing input file",
        "missing output file", "no such page", "unknown format: {e}", "note: {n}",
        "référence", "catégorie", "désignation", "descripción", "währung", "activation limit", "not active"}


def unescape(s):
    """A Rust string literal's text."""
    s = re.sub(r"\\\n\s*", "", s)  # line continuation
    s = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), s)
    return re.sub(r"\\(.)", lambda m: {"n": "\n", "t": "\t", "r": "\r", "0": "\0"}.get(m.group(1), m.group(1)), s)


def clean(src):
    """Source without tests, comments, raw strings and char literals (which confuse LIT)."""
    src = strip_tests(src)
    src = re.sub(r'r(#+)".*?"\1', '""', src, flags=re.S)
    src = re.sub(r"'(?:\\.|[^'\\])'", "' '", src)
    src = re.sub(r"(?m)^\s*//[^\n]*|(?<=[;{}),])\s*//[^\n]*", "", src)
    return src


def strip_tests(src):
    i = src.find("#[cfg(test)]")
    return src if i < 0 else src[:i]


def words(s):
    """Looks like text for people, not an id, key, colour, path or format string."""
    if not re.search(r"[A-Za-z]", s): return False
    if re.fullmatch(r"[a-z0-9_:\-./#]+", s): return False          # ids, icon names, keys
    if re.fullmatch(r"#?[0-9A-Fa-f]{6,8}", s): return False
    if re.fullmatch(r"[A-Z0-9_\-]+:?", s) and len(s) <= 4: return False  # tags, short codes
    if s.startswith(("http", "naqsha", "user:", "pro:", "x_", "{", "%", "$", "#!")): return False
    if "\n--\n" in s: return False                                              # UML / ER templates
    if re.fullmatch(r"[a-z]+[A-Z][A-Za-z0-9]*|[a-z]+[+-]", s): return False      # icon names (saveAs)
    if re.fullmatch(r"[A-Z][A-Z0-9_]+", s) and "_" in s: return False            # environment variables
    if re.fullmatch(r"[⌘⇧⌥↩⌫↑↓\s/A-Z0-9\[\]=+\-]+|Esc", s): return False      # shortcut keys alone
    if s in SKIP: return False
    if "\\" in s and "\n" not in s and "\\n" not in s: return False
    if re.search(r"[<>]|=\"|::|\.rs\b|\{[:?]|\{\}", s): return False  # markup, code, format! strings
    return True


def collect():
    out = []
    seen = set()

    def add(s, check=True):
        s = unescape(s)
        if s and s not in seen and (words(s) or not check and re.search(r"[A-Za-z]", s)):
            seen.add(s); out.append(s)

    for f in UI_FILES:
        p = os.path.join(ROOT, f)
        if not os.path.exists(p): continue
        src = clean(open(p, encoding="utf-8").read())
        # anything passed to tr() / t() is interface text, whatever it looks like
        for m in re.finditer(r'\b(?:tr|t|trf)\(\s*(?:if [^{]*\{\s*)?' + LIT, src):
            add(m.group(1), False)
        for m in re.finditer(LIT, src):
            add(m.group(1))
    # shape library: names, default text, categories
    lib = clean(open(os.path.join(ROOT, "src/library.rs"), encoding="utf-8").read())
    for m in re.finditer(r'd!\(\s*' + LIT + r'\s*,\s*' + LIT + r'\s*,[^,]*,[^,]*,[^,]*,\s*' + LIT, lib):
        add(m.group(2)); add(m.group(3))
    for m in re.finditer(r'CATEGORIES: &\[&str\] = &\[(.*?)\];', lib, re.S):
        for n in re.findall(LIT, m.group(1)): add(n)
    # templates: names, descriptions, categories
    for f in ["src/templates.rs", "pro/src/templates.rs"]:
        p = os.path.join(ROOT, f)
        if not os.path.exists(p): continue
        src = clean(open(p, encoding="utf-8").read())
        for m in re.finditer(r'\b(?:e|el|io|mf)\(\s*' + LIT + r'\s*,\s*' + LIT + r'\s*,\s*' + LIT + r'(?:\s*,\s*' + LIT + r')?', src):
            g = m.groups()
            add(g[1])
            # core templates: (id, name, cat, desc); Pro: (id, name, desc)
            if g[3] is not None: add(g[2]); add(g[3])
            else: add(g[2])
        for m in re.finditer(r'CATS: &\[&str\] = &\[(.*?)\];', src, re.S):
            for n in re.findall(LIT, m.group(1)): add(n)
    # Pro symbols and their groups
    p = os.path.join(ROOT, "pro/src/symbols.rs")
    if os.path.exists(p):
        src = clean(open(p, encoding="utf-8").read())
        for m in re.finditer(r's!\(\s*' + LIT + r'\s*,\s*' + LIT, src): add(m.group(2))
        for m in re.finditer(r'GROUPS: &\[&str\] = &\[(.*?)\];', src, re.S):
            for n in re.findall(LIT, m.group(1)): add(n)
    # names in other modules
    for f, pat in [("src/anim.rs", r'=> ' + LIT), ("src/anim.rs", r'\(\d\.\d, ' + LIT + r'\)'),
                   ("src/theme.rs", r'\(\s*' + LIT + r', "#'), ("src/files.rs", r'WINDOW: &str = if WEB \{ ' + LIT + r' \} else \{ ' + LIT)]:
        src = clean(open(os.path.join(ROOT, f), encoding="utf-8").read())
        for m in re.finditer(pat, src):
            for g in m.groups(): add(g)
    for s in ["System language", "Language", "lifetime", "active", "expired", "disabled", "inactive", "window", "tab",
              "glued", "free", "$8 / month", "$79 / year", "$129 once"]:
        add(json.dumps(s)[1:-1], False)
    return out


def load(code):
    p = os.path.join(ROOT, "locales", code + ".json")
    return json.load(open(p, encoding="utf-8")) if os.path.exists(p) else {}


def save(code, d):
    keys = collect()
    order = {k: i for i, k in enumerate(keys)}
    d = dict(sorted(d.items(), key=lambda kv: order.get(kv[0], 1e9)))
    with open(os.path.join(ROOT, "locales", code + ".json"), "w", encoding="utf-8") as f:
        json.dump(d, f, ensure_ascii=False, indent=0)
        f.write("\n")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
    if cmd == "list":
        json.dump(collect(), sys.stdout, ensure_ascii=False, indent=0)
    elif cmd == "missing":
        have = load(sys.argv[2])
        json.dump([k for k in collect() if not have.get(k)], sys.stdout, ensure_ascii=False, indent=0)
    elif cmd == "merge":
        d = load(sys.argv[2])
        d.update({k: v for k, v in json.load(open(sys.argv[3], encoding="utf-8")).items() if v})
        save(sys.argv[2], d)
    elif cmd == "todo":
        # numbered lines for translating: N|English (\n for line breaks), N counting every string; see "apply"
        have = load(sys.argv[2])
        keys = collect()
        lo, hi = int(sys.argv[3]), int(sys.argv[4])
        for i, k in enumerate(keys[lo:hi], lo):
            if not have.get(k): print(f"{i}|{k}".replace("\n", "\\n"))
    elif cmd == "apply":
        # N|translation lines (from "todo" numbering) for CODE, read from a file
        have = load(sys.argv[2])
        keys = collect()
        new = {}
        for line in open(sys.argv[3], encoding="utf-8"):
            line = line.rstrip("\n")
            if not line.strip(): continue
            n, _, t = line.partition("|")
            new[keys[int(n)]] = t.replace("\\n", "\n")
        have.update(new)
        save(sys.argv[2], have)
        print(f"{len(new)} added, {len([k for k in collect() if not have.get(k)])} still missing")
    elif cmd == "prune":
        keys = set(collect())
        for f in os.listdir(os.path.join(ROOT, "locales")):
            code = f[:-5]
            save(code, {k: v for k, v in load(code).items() if k in keys})
