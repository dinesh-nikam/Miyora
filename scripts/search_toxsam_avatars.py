import urllib.request
import json
import ssl

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

url = "https://raw.githubusercontent.com/ToxSam/open-source-avatars/main/data/avatars.json"
req = urllib.request.Request(url, headers={"User-Agent": "Miyora-Scout"})

with urllib.request.urlopen(req, context=ctx) as r:
    data = json.load(r)

print(f"Total avatars: {len(data)}")

# Let's inspect names and find anime, deity, and interesting avatars
found = []
for av in data:
    name = av.get("name", "")
    desc = av.get("description", "")
    model_url = av.get("model_file_url", "")
    thumb = av.get("thumbnail_url", "")
    if model_url and (model_url.endswith(".vrm") or av.get("format") == "VRM"):
        found.append({
            "name": name,
            "desc": desc,
            "url": model_url,
            "thumb": thumb
        })

print(f"Total VRM avatars with direct model_file_url: {len(found)}")
print("\nFirst 30 avatars in registry:")
for i, av in enumerate(found[:30]):
    print(f"{i+1}. {av['name']} -> {av['url']}")
