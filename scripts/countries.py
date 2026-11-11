#!/usr/bin/env python3
"""Writes core/src/countries_data.rs from two open data files:

    scripts/countries.py PhoneNumberMetadata.xml territories.json

PhoneNumberMetadata.xml: github.com/google/libphonenumber, resources/ (Apache License 2.0).
territories.json: github.com/unicode-org/cldr-json, cldr-localenames-full/main/en/ (Unicode License).
"""
import json
import sys
import xml.etree.ElementTree as ET

meta, names = sys.argv[1], sys.argv[2]
territories = json.load(open(names, encoding="utf-8"))["main"]["en"]["localeDisplayNames"]["territories"]
rows = []
for t in ET.parse(meta).getroot().iter("territory"):
    iso = t.get("id")
    if not (len(iso) == 2 and iso.isalpha()) or iso not in territories:
        continue
    prefix = t.get("internationalPrefix", "")
    if not prefix.isdigit():
        prefix = t.get("preferredInternationalPrefix", "")
    rows.append((territories[iso], iso, t.get("countryCode"), prefix, t.get("nationalPrefix", "")))
rows.sort(key=lambda r: r[0].casefold())
out = ["// Made by scripts/countries.py from libphonenumber's metadata and Unicode CLDR's names. Do not edit.", "",
       "use crate::countries::Country;", "", f"pub static COUNTRIES: [Country; {len(rows)}] = ["]
for name, iso, code, prefix, trunk in rows:
    out.append(f'    Country {{ name: {json.dumps(name, ensure_ascii=False)}, iso: "{iso}", code: "{code}", international_prefix: "{prefix}", trunk_prefix: "{trunk}" }},')
out.append("];")
open("core/src/countries_data.rs", "w", encoding="utf-8").write("\n".join(out) + "\n")
print(len(rows), "countries")
