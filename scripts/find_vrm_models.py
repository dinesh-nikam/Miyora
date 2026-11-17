import urllib.request
import json
import ssl

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

urls = [
    "https://api.github.com/repos/madjin/vrm-samples/contents/vroid/stable",
    "https://api.github.com/repos/madjin/vrm-samples/contents/vroid/beta",
    "https://api.github.com/repos/ToxSam/open-source-avatars/contents/data",
]

for url in urls:
    print(f"\n--- Fetching: {url} ---")
    req = urllib.request.Request(url, headers={"User-Agent": "Miyora-Scout"})
    try:
        with urllib.request.urlopen(req, context=ctx, timeout=10) as resp:
            data = json.load(resp)
            if isinstance(data, list):
                for item in data:
                    name = item.get("name", "")
                    size = item.get("size", 0)
                    dl = item.get("download_url", "")
                    if name.endswith(".vrm") or name.endswith(".glb") or name.endswith(".json"):
                        print(f"FOUND: {name} ({size} bytes) -> {dl}")
                    elif item.get("type") == "dir":
                        print(f"DIR: {name}")
    except Exception as e:
        print(f"Failed {url}: {e}")
