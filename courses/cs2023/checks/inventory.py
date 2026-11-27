"""Reproduce the official area/unit inventory from the report's linked table of contents."""
from hashlib import sha256
from html.parser import HTMLParser
from pathlib import Path
import re
import sys

REPORT = "https://csed.acm.org/wp-content/uploads/2025/11/CS2023-Report.htm"
DIGEST = "8fc7212255af71eb4458122ccb8a93052601e6a2fff59d7c2f4e2db18081b72e"

class Contents(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.anchor = None
        self.parts = []
    def handle_starttag(self, tag, attrs):
        if tag == "a":
            self.anchor = dict(attrs).get("href")
            self.parts = []
    def handle_data(self, text):
        if self.anchor:
            self.parts.append(text)
    def handle_endtag(self, tag):
        if tag == "a" and self.anchor:
            if self.anchor.startswith("#_Toc"):
                self.links.append((self.anchor, " ".join("".join(self.parts).split())))
            self.anchor = None

def extract(data):
    if sha256(data).hexdigest() != DIGEST:
        raise ValueError("Publisher bytes differ from pinned edition")
    toc = Contents()
    toc.feed(data.decode("cp1252"))
    areas = []
    for i, (anchor, label) in enumerate(toc.links):
        if not re.fullmatch(r"Knowledge Units\.? \d+", label):
            continue
        for a, title in reversed(toc.links[:i]):
            m = re.fullmatch(r"(.+?)\s*\(([A-Z]{2,4})\)\.? \d+", title)
            if m:
                break
        else:
            raise ValueError(title)
        area = dict(id=m[2], title=m[1], anchor=a, units=[])
        for u, text in toc.links[i+1:]:
            if text.startswith(("Professional Dispositions", "Mathematics Requirements", "Course Packaging")):
                break
            text = re.sub(r"\.? \d+$", "", text)
            name, sep, title = text.partition(":")
            area["units"].append(dict(id=name, title=title.strip() if sep else name, anchor=u))
        areas.append(area)
    return dict(report=REPORT, sha256=DIGEST, areas=areas)

if __name__ == "__main__":
    import tomllib
    catalog = tomllib.loads((Path(__file__).resolve().parents[1] / "topics.toml").read_text())
    expected = extract(Path(sys.argv[1]).read_bytes())
    actual = [dict(id=a["id"], title=a["title"], anchor=a["source"].removeprefix(REPORT), units=a["units"]) for a in catalog["areas"]]
    if catalog["report"] != REPORT or catalog["report_sha256"] != DIGEST or expected["areas"] != actual:
        sys.exit("Catalog differs from the official area/unit inventory")
    print(f"{len(actual)} areas and {sum(len(a['units']) for a in actual)} units match the edition, order, names and anchors.")
