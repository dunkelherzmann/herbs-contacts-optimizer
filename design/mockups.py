#!/usr/bin/env python3
"""Rough wireframe mockups for Herb's Contacts Optimizer (arrangement only)."""
import sys, os
from xml.sax.saxutils import escape

OUT = sys.argv[1]
W, H = 1200, 780
TB = 28          # title bar
TOOL = 52        # toolbar height
TOP = TB + TOOL  # content top
SIDE = 210       # sidebar width
STAT = 26        # status bar

LIGHT = dict(bg="#ffffff", title="#e4e4e4", bar="#f3f3f3", side="#ececec", line="#c8c8c8",
             btn="#dedede", text="#222", dim="#7a7a7a", stripe="#f6f6f6", sel="#9fb6d6",
             field="#ffffff", red="#c0392b", note="#e07b00", acc="#4a6fa5", card="#f7f7f7")
DARK = dict(bg="#1f1f1f", title="#383838", bar="#2b2b2b", side="#262626", line="#444",
            btn="#3d3d3d", text="#e6e6e6", dim="#9a9a9a", stripe="#252525", sel="#3d5a80",
            field="#151515", red="#e06c5f", note="#ffa940", acc="#7aa2d8", card="#2a2a2a")


NARROW = set("iljtfr.,()'|1I: ")
WIDE = set("mwMW")


def text_w(label, size=13):
    w = 0.0
    for ch in label:
        if ch in NARROW:
            w += 3.9
        elif ch in WIDE:
            w += 10.6
        elif ch.isupper():
            w += 8.6
        else:
            w += 7.0
    return w * size / 13


def btn_w(label, chev=False):
    return text_w(label) + 28 + (14 if chev else 0)


class S:
    def __init__(s, c, w=W, h=H):
        s.c, s.w, s.h, s.o = c, w, h, []

    def r(s, x, y, w, h, fill=None, stroke=None, rx=0, dash=False, sw=1):
        f = fill or "none"
        st = f' stroke="{stroke}" stroke-width="{sw}"' if stroke else ""
        d = ' stroke-dasharray="5 4"' if dash else ""
        s.o.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{f}"{st}{d}/>')

    def t(s, x, y, txt, size=13, col=None, anchor="start", bold=False, italic=False):
        col = col or s.c["text"]
        b = ' font-weight="600"' if bold else ""
        i = ' font-style="italic"' if italic else ""
        s.o.append(f'<text x="{x}" y="{y}" font-size="{size}" fill="{col}" text-anchor="{anchor}"{b}{i}>{escape(txt)}</text>')

    def l(s, x1, y1, x2, y2, col=None, sw=1):
        s.o.append(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{col or s.c["line"]}" stroke-width="{sw}"/>')

    def circ(s, x, y, r, fill, stroke=None):
        st = f' stroke="{stroke}"' if stroke else ""
        s.o.append(f'<circle cx="{x}" cy="{y}" r="{r}" fill="{fill}"{st}/>')

    def btn(s, x, y, label, w=None, h=26, primary=False, danger=False, disabled=False, chev=False):
        w = w or btn_w(label, chev)
        fill = s.c["acc"] if primary else s.c["btn"]
        col = "#fff" if primary else (s.c["red"] if danger else (s.c["dim"] if disabled else s.c["text"]))
        s.r(x, y, w, h, fill, s.c["red"] if danger else None, rx=6)
        s.t(x + (w - (12 if chev else 0)) / 2, y + h / 2 + 4.5, label, 13, col, "middle")
        if chev:
            s.t(x + w - 13, y + h / 2 + 4, "▾", 12, col, "middle")
        return x + w

    def btns_right(s, xr, y, items, gap=6):
        """Buttons laid out from the right edge xr; items are (label, kwargs)."""
        ws = [kw.get("w") or btn_w(lab, kw.get("chev", False)) for lab, kw in items]
        x = xr - sum(ws) - gap * (len(ws) - 1)
        x0 = x
        for (lab, kw), w in zip(items, ws):
            kw = dict(kw); kw["w"] = w
            s.btn(x, y, lab, **kw)
            x += w + gap
        return x0

    def field(s, x, y, w, txt="", h=26, ph=False):
        s.r(x, y, w, h, s.c["field"], s.c["line"], rx=5)
        if txt:
            s.t(x + 8, y + h / 2 + 4.5, txt, 13, s.c["dim"] if ph else s.c["text"])

    def tick(s, x, y, on=False):
        s.r(x, y, 14, 14, s.c["acc"] if on else s.c["field"], s.c["line"], rx=3)
        if on:
            s.o.append(f'<path d="M{x+3} {y+7} l3 3 l5 -6" stroke="#fff" stroke-width="2" fill="none"/>')

    def avatar(s, x, y, r, initials):
        s.circ(x, y, r, s.c["btn"], s.c["line"])
        s.t(x, y + r * 0.35, initials, int(r * 0.9), s.c["dim"], "middle")

    def n(s, x, y, num):
        """Annotation badge: row number in design/references.md."""
        s.circ(x, y, 9, s.c["note"])
        s.t(x, y + 4, str(num), 10, "#fff", "middle", bold=True)

    def save(s, name):
        body = "\n".join(s.o)
        svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{s.w}" height="{s.h}" viewBox="0 0 {s.w} {s.h}" '
               f'font-family="Helvetica, Arial, sans-serif">\n<rect width="100%" height="100%" fill="{s.c["bg"]}"/>\n{body}\n</svg>\n')
        open(os.path.join(OUT, name), "w").write(svg)


# ---------- shared chrome ----------

def chrome(s, disabled_tools=False, filter_count=0, dup=3, open_ok=True):
    c = s.c
    # title bar: smallest native one, app name only (Mac shown; other systems use their own)
    s.r(0, 0, W, TB, c["title"])
    for i, col in enumerate(["#ff5f57", "#febc2e", "#28c840"]):
        s.circ(16 + i * 20, TB / 2, 6, col)
    s.t(W / 2, 19, "Herb's Contacts Optimizer", 13, c["text"], "middle", bold=True)
    s.n(W / 2 + 105, 14, 1)
    s.l(0, TB, W, TB)
    # toolbar
    s.r(0, TB, W, TOOL, c["bar"])
    y = TB + 13
    x = 12
    d = disabled_tools
    # Main Menu first, at the left, never moved (his request for this project)
    s.r(x, y, 34, 26, c["btn"], rx=6)             # ☰ drawn with the painter
    for k in range(3):
        s.l(x + 10, y + 8 + k * 5, x + 24, y + 8 + k * 5, c["text"], 2)
    s.n(x + 2, y - 6, 5)
    x += 34 + 16
    s.n(x + 2, y - 6, 2)
    x = s.btn(x, y, "Open", disabled=not open_ok) + 6
    x = s.btn(x, y, "Save", disabled=d) + 6
    x = s.btn(x, y, "Export", disabled=d) + 16
    x0 = x
    x = s.btn(x, y, f"Find Duplicates ({dup})" if dup else "Find Duplicates", disabled=d) + 6
    s.n(x0 + 150, y - 6, 3)
    x = s.btn(x, y, "Wizard", disabled=d) + 6
    x = s.btn(x, y, "Toolbox", disabled=d) + 16
    x = s.btn(x, y, "New Contact", disabled=d) + 6
    # right group: Filter, Search, Keep on Top
    px = W - 12 - 34
    s.r(px, y, 34, 26, c["btn"], rx=6)            # Keep on Top pin
    s.circ(px + 17, y + 10, 4, c["text"])
    s.l(px + 17, y + 14, px + 17, y + 21, c["text"], 2)
    s.n(px + 30, y - 6, 4)
    sx = px - 8 - 200
    s.field(sx, y, 200, "Search", ph=True)
    if d and not open_ok:
        s.r(sx, y, 200, 26, c["bar"], rx=5); s.o[-1] = s.o[-1].replace("/>", ' fill-opacity="0.6"/>')
    fx = sx - 8 - (96 if filter_count else 70)
    s.btn(fx, y, f"Filter ({filter_count})" if filter_count else "Filter", w=(96 if filter_count else 70),
          primary=bool(filter_count), disabled=d and not filter_count)
    s.n(fx + 2, y - 6, 14)
    s.l(0, TOP, W, TOP)


def sidebar(s, books=(("Family.vcf", 6, True, False), ("Work.csv", 1, False, False), ("Old phone.vcf", 214, False, True))):
    c = s.c
    s.r(0, TOP, SIDE, H - TOP - STAT, c["side"])
    s.l(SIDE, TOP, SIDE, H - STAT)
    s.t(14, TOP + 24, "ADDRESS BOOKS", 11, c["dim"], bold=True)
    s.n(SIDE - 18, TOP + 20, 6)
    y = TOP + 36
    for name, cnt, sel, ro in books:
        if sel:
            s.r(6, y, SIDE - 12, 26, c["sel"], rx=5)
        s.t(18, y + 18, name + ("  •" if sel else ""), 13)
        if ro:
            s.t(18 + len(name) * 7 + 6, y + 18, "(read-only)", 11, c["dim"])
        s.t(SIDE - 16, y + 18, str(cnt), 12, c["dim"], "end")
        y += 30
    s.t(18, y + 20, "Open a file…", 13, c["acc"])
    # bottom of sidebar: Merge Address Books (enabled when 2+ chosen)
    s.btn(12, H - STAT - 40, "Merge Address Books", w=SIDE - 24, disabled=True)


def status(s, left, right=""):
    c = s.c
    s.r(0, H - STAT, W, STAT, c["bar"])
    s.l(0, H - STAT, W, H - STAT)
    s.t(14, H - 8, left, 12, c["dim"])
    s.n(14 + len(left) * 6.4 + 16, H - 13, 13)
    if right:
        s.t(W - 14, H - 8, right, 12, c["dim"], "end")


CONTACTS = [
    ("AG", "Ana García", "+34 600 111 222", "ana@example.org"),
    ("BS", "Bob Stone", "555 0100", "bob@example.org"),
    ("CW", "Chen Wei", "+86 10 5550 0100", "chen@example.cn"),
    ("CW", "chen  WEI", "+86 10 55500100", "Chen@example.cn"),
    ("DM", "Dana Müller", "0170 1234567", ""),
    ("BS", "Bob Stone", "555-0100", "bob@example.org"),
]


def contact_list(s, x, y, w, h, rows=CONTACTS, selected=0, ticked=(), bar=True):
    c = s.c
    # header: tick-all, avatar, Name (sorted), Phone, E-mail
    s.tick(x + 12, y + 9)
    s.t(x + 74, y + 21, "Name ▲", 12, c["dim"])
    pcol = x + 74 + int((w - 74) * 0.40)
    ecol = x + 74 + int((w - 74) * 0.68)
    s.t(pcol, y + 21, "Phone", 12, c["dim"])
    s.t(ecol, y + 21, "E-mail", 12, c["dim"])
    s.n(x + 50, y + 16, 7)
    s.l(x, y + 32, x + w, y + 32)
    ry = y + 33
    for i, (ini, name, ph, em) in enumerate(rows):
        if i == selected:
            s.r(x + 4, ry, w - 8, 36, c["sel"], rx=4)
        elif i % 2:
            s.r(x, ry, w, 36, c["stripe"])
        s.tick(x + 12, ry + 11, i in ticked)
        s.avatar(x + 50, ry + 18, 12, ini)
        s.t(x + 74, ry + 23, name, 13)
        s.t(pcol, ry + 23, ph, 13, c["dim"])
        maxch = int((x + w - ecol - 10) / 7.0)
        em2 = em if len(em) <= maxch else em[: max(maxch - 1, 3)] + "…"
        s.t(ecol, ry + 23, em2, 13, c["dim"])
        ry += 36
    if len(rows) > 3:
        s.n(x + 200, y + 33 + 36 * 3 + 18, 8)
    else:
        s.n(x + 50, y + 33 + 2, 8)
    if bar and ticked:
        by = y + h - 44
        s.r(x, by, w, 44, c["bar"])
        s.l(x, by, x + w, by)
        s.t(x + 14, by + 27, f"{len(ticked)} ticked", 13, bold=True)
        bx = x + 90
        bx = s.btn(bx, by + 9, "Copy To…") + 6
        bx = s.btn(bx, by + 9, "Move To…") + 6
        bx = s.btn(bx, by + 9, "Edit Ticked…") + 6
        bx = s.btn(bx, by + 9, "Delete", danger=True) + 6
        s.btn(x + w - 110, by + 9, "Deselect All", w=98)
        s.n(x + 80, by + 8, 9)


def details(s, x, y, w, h):
    c = s.c
    s.btn(x + w - 150, y + 14, "Edit", w=60)
    s.btn(x + w - 82, y + 14, "Delete", w=66)
    s.avatar(x + 52, y + 80, 34, "AG")
    s.n(x + 90, y + 54, 10)
    s.t(x + 100, y + 78, "Ana García", 22)
    s.t(x + 100, y + 100, "Manager, Acme", 13, c["dim"])
    yy = y + 145
    for head, rows in (("PHONES", [("mobile", "+34 600 111 222"), ("work", "+34 910 000 000")]),
                       ("E-MAILS", [("home", "ana@example.org")]),
                       ("ADDRESSES", [("home", "Calle Mayor 1, 28001 Madrid")]),
                       ("OTHER", [("birthday", "1980-02-03"), ("groups", "Friends, Work")]),
                       ("NOTE", [("", "Met at the conference in Lisbon.")])):
        s.t(x + 18, yy, head, 11, c["dim"], bold=True)
        yy += 22
        for k, v in rows:
            s.t(x + 100, yy, k, 13, c["dim"], "end")
            s.t(x + 112, yy, v, 13)
            yy += 22
        yy += 12


# ---------- windows ----------

def main_window(c, name):
    s = S(c)
    chrome(s)
    sidebar(s)
    lx, lw = SIDE, 600
    contact_list(s, lx, TOP, lw, H - TOP - STAT, selected=0, ticked=(1, 5))
    s.l(lx + lw, TOP, lx + lw, H - STAT)
    details(s, lx + lw, TOP, W - lx - lw, H - TOP - STAT)
    status(s, "6 contacts, 2 ticked, unsaved changes", "Opened Work.csv with 1 contact.")
    s.save(name)


def empty_window():
    s = S(LIGHT)
    chrome(s, disabled_tools=True, dup=0)
    sidebar(s, books=())
    c = s.c
    cx = SIDE + (W - SIDE) / 2
    s.r(cx - 40, 280, 80, 80, c["btn"], c["line"], rx=18, dash=True)
    s.t(cx, 325, "app icon", 11, c["dim"], "middle")
    s.t(cx, 400, "Open a contacts file to begin", 20, anchor="middle")
    s.t(cx, 426, "vCard, CSV, Excel, LDIF and more. You can also drop files on this window.", 13, c["dim"], "middle")
    s.btn(cx - 50, 446, "Open…", w=100, primary=True)
    s.n(cx + 70, 300, 15)
    status(s, "No address book open")
    s.save("empty.svg")


def filters_window():
    s = S(LIGHT)
    c = s.c
    chrome(s, filter_count=2)
    sidebar(s)
    x, w = SIDE, W - SIDE
    # filter bar: one row per filter
    s.r(x, TOP, w, 118, c["card"])
    s.l(x, TOP + 118, x + w, TOP + 118)
    y = TOP + 12
    for a, b, v, isfield in (("Text", "Does not contain", "lisbon", True), ("Data", "Lacks", "an e-mail address", False)):
        xx = s.btn(x + 14, y, a, w=110, chev=True) + 8
        xx = s.btn(xx, y, b, w=170, chev=True) + 8
        if isfield:
            s.field(xx, y, 260, v)
        else:
            s.btn(xx, y, v, w=260, chev=True)
        s.btn(xx + 268, y, "−", w=30)
        y += 34
    xx = s.btn(x + 14, y + 4, "Add Filter") + 8
    xx = s.btn(xx, y + 4, "Clear Filters") + 8
    s.btn(x + w - 80, y + 4, "Done", w=66)
    s.n(x + w - 100, TOP + 20, 14)
    # list without details pane (filters shown, nothing selected)
    rows = [CONTACTS[4]]
    contact_list(s, x, TOP + 119, 600, H - TOP - STAT - 119, rows=rows, selected=-1, ticked=())
    s.l(x + 600, TOP + 119, x + 600, H - STAT)
    # no-selection state in details pane
    s.t(x + 600 + (w - 600) / 2, 400, "No contact selected", 15, c["dim"], "middle")
    status(s, "1 of 6 contacts", "")
    s.save("filters.svg")


def work_area_header(s, kicker, title, line, back=None):
    c = s.c
    x = SIDE + 24
    y = TOP + 24
    if back:
        s.btn(x, y - 4, back)
        y += 32
    if kicker:
        s.t(x, y + 10, kicker, 11, c["dim"], bold=True)
        y += 18
    s.t(x, y + 18, title, 20)
    s.t(x, y + 42, line, 13, c["dim"])
    return y + 58


def toolbox_step():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s)
    x0, x1 = SIDE + 24, W - 24
    y = work_area_header(s, "", "Merge Contacts", "Each line is a set of similar contacts. Ticked sets become one contact.", back="‹ Toolbox")
    s.n(SIDE + 12, TOP + 16, 16)
    s.t(x0, y + 14, "1 of 2 ticked", 12, c["dim"])
    hx = s.btns_right(x1, y, [("Select All", {}), ("Deselect All", {}), ("One at a Time", {})])
    s.n(hx - 10, y - 4, 17)
    y += 36
    s.l(x0, y, x1, y)
    for i, (on, a, b) in enumerate(((True, "Bob Stone", "+ Bob Stone (555-0100)"), (False, "Chen Wei", "+ chen  WEI"))):
        ry = y + 8 + i * 40
        s.tick(x0, ry + 6, on)
        s.avatar(x0 + 34, ry + 13, 11, a.split()[0][0] + a.split()[-1][0])
        s.t(x0 + 54, ry + 18, a, 13)
        s.t(x0 + 190, ry + 18, b, 13, c["dim"])
        rx0 = s.btns_right(x1, ry, [(f"Into {a}", {"w": 180, "chev": True}), ("Preview", {"primary": i == 0}),
                                    ("Edit", {}), ("Delete", {"danger": True})])
    s.n(rx0 - 10, y + 8, 18)
    y += 92
    s.l(x0, y, x1, y)
    # preview of the merged contact, laid out like the details pane
    s.t(x0, y + 22, "MERGED CONTACT, FROM 2", 11, c["dim"], bold=True)
    s.t(x1, y + 22, "+ what the others add", 12, c["acc"], "end")
    s.avatar(x0 + 26, y + 62, 24, "BS")
    s.t(x0 + 60, y + 68, "Bob Stone", 18)
    yy = y + 110
    for k, v, add in (("mobile", "555 0100", False), ("work", "+ 555 0199", True), ("home", "bob@example.org", False), ("birthday", "+ 1975-06-01", True)):
        s.t(x0 + 90, yy, k, 13, c["dim"], "end")
        s.t(x0 + 102, yy, v, 13, c["acc"] if add else c["text"])
        yy += 22
    s.n(x0 + 200, y + 18, 19)
    # footer
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btn(x1 - 150, fy + 11, "Close", w=70)
    s.btn(x1 - 72, fy + 11, "Merge", w=72, primary=True)
    s.n(x1 - 160, fy + 10, 20)
    status(s, "6 contacts", "")
    s.save("toolbox-step.svg")


def steps_strip(s, y, cur, total=11):
    y = TOP + 6
    c = s.c
    x0, x1 = SIDE + 24, W - 24
    seg = (x1 - x0 - (total - 1) * 4) / total
    for i in range(total):
        col = c["acc"] if i < cur else (c["note"] if False else c["btn"])
        s.r(x0 + i * (seg + 4), y, seg, 6, col, rx=3)


def wizard_step():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s, books=(("Messy.vcf", 5, True, False),))
    x0, x1 = SIDE + 24, W - 24
    steps_strip(s, TOP + 14, 3)
    s.n(x1 - 4, TOP + 16, 21)
    y = work_area_header(s, "STEP 3 OF 11", "Similar Contacts", "These contacts share a name, a phone number or an e-mail address. Tick the ones to delete.")
    s.t(x0, y + 14, "0 of 2 ticked", 12, c["dim"])
    hx = s.btns_right(x1, y, [("Select All", {}), ("Deselect All", {}), ("One at a Time", {})])
    s.n(hx - 10, y - 4, 17)
    y += 36
    s.l(x0, y, x1, y)
    for i, n in enumerate(("bob stone", "Stone Bob")):
        ry = y + 8 + i * 40
        s.tick(x0, ry + 6)
        s.avatar(x0 + 34, ry + 13, 11, "BS")
        s.t(x0 + 54, ry + 18, n, 13)
        bx = s.btns_right(x1, ry, [("Edit", {}), ("Delete", {"danger": True})])
        s.t(bx - 14, ry + 18, "bob@example.org", 13, c["dim"], "end")
    s.n(x1 - 330, y + 8, 36)
    y += 92
    s.l(x0, y, x1, y)
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btn(x0, fy + 11, "Cancel Wizard")
    fx = s.btns_right(x1, fy + 11, [("Skip", {}), ("Delete Contacts", {"danger": True, "disabled": True})])
    s.n(fx - 10, fy + 10, 22)
    status(s, "5 contacts", "")
    s.save("wizard-step.svg")


def wizard_preview():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s, books=(("Messy.vcf", 3, True, False),))
    x0, x1 = SIDE + 24, W - 24
    steps_strip(s, TOP + 14, 11)
    y = work_area_header(s, "BEFORE YOU KEEP THEM", "Changes", "The address book as the wizard leaves it. Pick a contact to see what changed.")
    # show: All / Changed / Deleted  (5 before: 3 kept, all 3 changed; 2 gone)
    bx = x0
    bx = s.btn(bx, y, "All 5") + 6
    bx = s.btn(bx, y, "Changed 3", primary=True) + 6
    bx = s.btn(bx, y, "Deleted 2")
    s.n(bx + 14, y + 2, 23)
    y += 38
    lw = 520
    rows = [("changed", "AG", "Ana García", "was ANA GARCÍA"), ("merged", "BS", "Bob Stone", "+ Stone Bob"),
            ("changed", "I", "info", "name filled in")]
    for i, (k, ini, n, why) in enumerate(rows):
        ry = y + i * 38
        if i == 1:
            s.r(x0 - 4, ry, lw, 36, c["sel"], rx=4)
        mark = {"changed": "●", "merged": "+", "deleted": "−"}[k]
        s.t(x0 + 6, ry + 23, mark, 15, c["red"] if k == "deleted" else c["acc"], "middle", bold=True)
        s.avatar(x0 + 36, ry + 18, 12, ini)
        s.t(x0 + 58, ry + 23, n, 13)
        s.t(x0 + lw - 16, ry + 23, why, 12, c["dim"], "end")
    s.n(x0 + 20, y - 4, 24)
    # the chosen contact after the run, with the details pane's header
    px = x0 + lw + 20
    s.l(px - 10, y - 6, px - 10, H - STAT - 60)
    s.t(px + 8, y + 12, "AFTER THE RUN", 11, c["dim"], bold=True)
    s.avatar(px + 40, y + 60, 30, "BS")
    s.t(px + 82, y + 58, "Bob Stone", 20)
    s.t(px + 82, y + 78, "was bob stone; Stone Bob merged in", 12, c["dim"])
    yy = y + 124
    for k, v, add in (("mobile", "555 0100", False), ("mobile", "+ 555 0199", True), ("home", "bob@example.org", False), ("note", "+ met in Lisbon", True)):
        s.t(px + 80, yy, k, 13, c["dim"], "end")
        s.t(px + 92, yy, v, 13, c["acc"] if add else c["text"])
        yy += 22
    s.n(px + 150, y + 8, 25)
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btn(x0, fy + 11, "Undo All")
    s.btns_right(x1, fy + 11, [("‹ Back", {}), ("Keep Changes", {"primary": True})])
    status(s, "5 contacts before, 3 after", "")
    s.save("wizard-preview.svg")


def wizard_summary():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s, books=(("Messy.vcf", 3, True, False),))
    x0, x1 = SIDE + 24, W - 24
    y = work_area_header(s, "", "Summary", "The file changes only when you save.")
    # three figures
    for i, (num, lab) in enumerate((("5", "before"), ("3", "after"), ("3", "modified"))):
        bx = x0 + i * 150
        s.r(bx, y, 136, 60, c["card"], c["line"], rx=8)
        s.t(bx + 68, y + 32, num, 24, anchor="middle")
        s.t(bx + 68, y + 50, lab, 12, c["dim"], "middle")
    s.n(x0 + 440, y + 10, 26)
    y += 84
    cols = ["Found", "Deleted", "Merged", "Repaired", "Added"]
    cx = [x0 + 330 + i * 110 for i in range(5)]
    for i, h in enumerate(cols):
        s.t(cx[i], y, h, 12, c["dim"], "end")
    s.n(cx[-1] + 20, y - 4, 27)
    y += 8
    s.l(x0, y, x1, y)
    data = [("Duplicates", None), ("Inner Duplicates", (1, 0, 0, 1, 0)), ("Similar Contacts", "Skipped"),
            ("Merge Contacts", (1, 0, 1, 0, 0)), ("Invalid Contacts", (1, 1, 0, 0, 0)), ("Missing Information", (1, 0, 0, 0, 0)),
            ("Fill In Empty Names", (1, 0, 0, 0, 1)), ("Tidy Names", None), ("Capitalization", (2, 0, 0, 2, 0)),
            ("Country Codes", "Skipped"), ("Simplify Numbers", "Skipped")]
    for i, (n, v) in enumerate(data):
        ry = y + 4 + i * 30
        if i % 2:
            s.r(x0, ry, x1 - x0, 30, c["stripe"])
        s.t(x0 + 8, ry + 20, n, 13, c["dim"] if not isinstance(v, tuple) else c["text"])
        if v is None:
            s.t(cx[0], ry + 20, "nothing found", 12, c["dim"], "end")
        elif v == "Skipped":
            s.t(cx[0], ry + 20, "skipped", 12, c["dim"], "end")
        else:
            for j, val in enumerate(v):
                s.t(cx[j], ry + 20, str(val) if val else "–", 13, c["text"] if val else c["dim"], "end")
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btn(x0, fy + 11, "Undo All")
    s.btn(x1 - 80, fy + 11, "Done", w=80, primary=True)
    status(s, "3 contacts, unsaved changes", "")
    s.save("wizard-summary.svg")


def editor():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s)
    lx, lw = SIDE, 360
    # narrower list stays visible beside the editor
    rows = [(a, b, "", "") for a, b, _, _ in CONTACTS]
    cc = s.c
    s.tick(lx + 12, TOP + 9)
    s.t(lx + 74, TOP + 21, "Name ▲", 12, cc["dim"])
    s.l(lx, TOP + 32, lx + lw, TOP + 32)
    for i, (ini, nm, _, _) in enumerate(rows):
        ry = TOP + 33 + i * 36
        if i == 0:
            s.r(lx + 4, ry, lw - 8, 36, cc["sel"], rx=4)
        s.tick(lx + 12, ry + 11)
        s.avatar(lx + 50, ry + 18, 12, ini)
        s.t(lx + 74, ry + 23, nm, 13)
    s.l(lx + lw, TOP, lx + lw, H - STAT)
    x = lx + lw
    w = W - x
    # pane header: Cancel / Save Contact
    s.t(x + 18, TOP + 30, "Edit Contact", 17)
    s.btn(W - 210, TOP + 12, "Cancel", w=70)
    s.btn(W - 132, TOP + 12, "Save Contact", w=118, primary=True)
    s.n(W - 222, TOP + 12, 28)
    # picture beside the name fields
    y = TOP + 56
    s.avatar(x + 60, y + 50, 40, "AG")
    s.btn(x + 20, y + 100, "Change…", w=80)
    s.n(x + 100, y + 14, 29)
    fx = x + 150
    fw = w - 150 - 20
    pairs = [("Prefix", "", "First name", "Ana"), ("Middle name", "", "Last name", "García"),
             ("Suffix", "", "Nickname", ""), ("Display name", "Ana García", None, None)]
    half = (fw - 20) / 2
    for i, (a, av, b, bv) in enumerate(pairs):
        yy = y + i * 46
        s.t(fx, yy + 10, a, 11, c["dim"])
        if b is None:
            s.field(fx, yy + 14, fw, av, h=24)
        else:
            s.field(fx, yy + 14, half, av, h=24)
            s.t(fx + half + 20, yy + 10, b, 11, c["dim"])
            s.field(fx + half + 20, yy + 14, half, bv, h=24)
    yy = y + len(pairs) * 46 + 4
    bx = s.btn(fx, yy, "Flatten", disabled=True) + 6
    s.btn(bx, yy, "Split")
    y = yy + 40
    # repeatable groups, each with its own + Add
    groups = [
        ("PHONES", [["mobile ▾", "+34 600 111 222", "+34 ▾"]], "Add Phone"),
        ("E-MAILS", [["home ▾", "ana@example.org"]], "Add E-mail"),
        ("INSTANT MESSAGING", [["Signal ▾", "ana.garcia"]], "Add Account Name"),
        ("ORGANIZATIONS", [["Acme", "Sales", "Manager"]], "Add Organization"),
        ("RELATIVES", [["sister ▾", "Lucía García"]], "Add Relative"),
    ]
    for g, (head, rows_, add) in enumerate(groups):
        s.t(x + 18, y + 12, head, 11, c["dim"], bold=True)
        y += 20
        for r_ in rows_:
            xx = x + 18
            widths = {3: [110, w - 18 - 110 - 90 - 36 - 30, 90], 2: [110, w - 18 - 110 - 36 - 24]}[len(r_)]
            if head == "ORGANIZATIONS":
                widths = [(w - 36 - 50) / 3] * 3
            for k, v in enumerate(r_):
                s.field(xx, y, widths[k], v, h=24)
                xx += widths[k] + 6
            s.btn(xx, y - 1, "−", w=28, h=26)
            y += 30
        s.t(x + 18, y + 12, "+ " + add, 12, c["acc"])
        y += 26
        if head == "INSTANT MESSAGING":
            s.n(x + 190, y - 70, 30)
        if head == "ORGANIZATIONS":
            s.n(x + 170, y - 70, 31)
        if head == "RELATIVES":
            s.n(x + 120, y - 70, 32)
    # the next group runs on under the status line: the pane scrolls
    s.t(x + 18, y + 12, "ADDRESSES", 11, c["dim"], bold=True)
    s.field(x + 18, y + 20, 110, "home ▾", h=24)
    s.field(x + 134, y + 20, w - 134 - 54, "Calle Mayor 1", h=24)
    s.r(W - 8, TOP + 56, 4, 220, c["line"], rx=2)
    status(s, "6 contacts", "")
    s.save("editor.svg")


def copy_to():
    s = S(LIGHT)
    c = s.c
    chrome(s)
    sidebar(s)
    contact_list(s, SIDE, TOP, 600, H - TOP - STAT, selected=0, ticked=(1, 5))
    s.l(SIDE + 600, TOP, SIDE + 600, H - STAT)
    details(s, SIDE + 600, TOP, W - SIDE - 600, H - TOP - STAT)
    status(s, "6 contacts, 2 ticked, unsaved changes", "")
    # dialog, light veil (not a heavy dim)
    s.r(0, 0, W, H, "#000000")
    s.o[-1] = s.o[-1].replace('fill="#000000"', 'fill="#000000" fill-opacity="0.2"')
    dx, dy, dw, dh = W / 2 - 210, 220, 420, 270
    s.r(dx, dy, dw, dh, c["card"], c["line"], rx=12)
    s.t(dx + 24, dy + 36, "Copy 2 Contacts To", 17)
    s.n(dx + dw - 24, dy + 30, 33)
    yy = dy + 64
    for i, (n, ro) in enumerate((("Work.csv", False), ("Old phone.vcf", True), ("New Address Book", False))):
        s.circ(dx + 34, yy + 8, 7, c["field"], c["line"])
        if i == 0:
            s.circ(dx + 34, yy + 8, 3.5, c["acc"])
        s.t(dx + 50, yy + 13, n + ("  (read-only)" if ro else ""), 13, c["dim"] if ro else c["text"])
        yy += 32
    s.t(dx + 24, yy + 14, "Family.vcf keeps them; Move To takes them out of it.", 12, c["dim"])
    s.btn(dx + dw - 190, dy + dh - 46, "Cancel", w=76)
    s.btn(dx + dw - 106, dy + dh - 46, "Copy", w=82, primary=True)
    s.save("copy-to.svg")


TOOLS = [
    ("DUPLICATES AND MERGING", [("Duplicates", "Delete identical contacts"),
                                ("Inner Duplicates", "Remove values repeated inside a contact"),
                                ("Similar Contacts", "Find contacts that are probably the same person"),
                                ("Merge Contacts", "Join similar contacts into one"),
                                ("Merge by Hand", "Pick the contacts to join yourself")]),
    ("REPAIRS", [("Invalid Contacts", "Find contacts that look wrong"),
                 ("Missing Information", "Find contacts without a name or a way to reach them"),
                 ("Fill In Empty Names", "Name contacts that have none"),
                 ("Tidy Names", "Join every name into the Name field, or split it into parts"),
                 ("Capitalization", "Fix names in all capitals or all small letters"),
                 ("Country Codes", "Put a country code in front of numbers that have none"),
                 ("Simplify Numbers", "Strip spaces and punctuation from phone numbers"),
                 ("Search and Replace", "Replace a text in all contacts")]),
    ("MASS DELETE", [("Delete Contacts", "Delete the contacts you tick"),
                     ("Delete Data", "Delete one kind of data from the contacts you tick")]),
]


def toolbox():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s)
    x0, x1 = SIDE + 24, W - 24
    y = work_area_header(s, "", "Toolbox", "Each tool shows what it would change before anything happens.")
    s.n(SIDE + 12, TOP + 16, 16)
    y -= 6
    for head, tools in TOOLS:
        s.t(x0, y + 14, head, 11, c["dim"], bold=True)
        y += 20
        for i, (name, line) in enumerate(tools):
            if i % 2:
                s.r(x0 - 6, y, x1 - x0 + 12, 26, c["stripe"], rx=4)
            s.t(x0 + 4, y + 18, name, 13)
            s.t(x0 + 190, y + 18, line, 13, c["dim"])
            s.t(x1 - 6, y + 18, "›", 16, c["dim"], "end")
            y += 26
        y += 4
    s.n(x0 + 172, TOP + 112, 34)
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btns_right(x1, fy + 11, [("Close", {})])
    status(s, "6 contacts", "")
    s.save("toolbox.svg")


WIZ = ["Duplicates", "Inner Duplicates", "Similar Contacts", "Merge Contacts", "Invalid Contacts", "Missing Information",
       "Fill In Empty Names", "Tidy Names", "Capitalization", "Country Codes", "Simplify Numbers"]


def wizard_start():
    s = S(LIGHT)
    c = s.c
    chrome(s, disabled_tools=True, open_ok=False)
    sidebar(s, books=(("Messy.vcf", 5, True, False),))
    x0, x1 = SIDE + 24, W - 24
    steps_strip(s, TOP + 14, 0)
    y = work_area_header(s, "", "Optimization Wizard", "Goes through the ticked steps one after another; you can undo the whole run at the end.")
    s.t(x0, y + 14, "11 of 11 ticked", 12, c["dim"])
    hx = s.btns_right(x1, y, [("Select All", {}), ("Deselect All", {})])
    s.n(hx - 10, y - 4, 17)
    y += 36
    s.l(x0, y, x1, y)
    lines = dict(l for _, t in TOOLS for l in t)
    for i, name in enumerate(WIZ):
        ry = y + 6 + i * 30
        if i % 2:
            s.r(x0 - 6, ry, x1 - x0 + 12, 30, c["stripe"], rx=4)
        s.tick(x0, ry + 8, True)
        s.t(x0 + 26, ry + 20, name, 13)
        s.t(x0 + 210, ry + 20, lines[name], 13, c["dim"])
    s.n(x0 + 190, y + 12, 35)
    fy = H - STAT - 48
    s.l(SIDE, fy, W, fy)
    s.btns_right(x1, fy + 11, [("Cancel", {}), ("Start", {"primary": True})])
    status(s, "5 contacts", "")
    s.save("wizard-start.svg")


main_window(LIGHT, "main-light.svg")
main_window(DARK, "main-dark.svg")
empty_window()
filters_window()
toolbox()
toolbox_step()
wizard_start()
wizard_step()
wizard_preview()
wizard_summary()
editor()
copy_to()
