import urllib.request
import json
import ssl

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

url = "https://raw.githubusercontent.com/ToxSam/open-source-avatars/main/data/avatars.json"
req = urllib.request.Request(url, headers={"User-Agent": "Miyora-Scout"})
try:
    with urllib.request.urlopen(req, context=ctx, timeout=10) as resp:
        data = json.load(resp)
        print(f"Total avatars in ToxSam registry: {len(data)}")
        for i, item in enumerate(data[:15]):
            print(f"[{i}] {item.get('name')} | {item.get('author')} | {item.get('license')} | files: {item.get('files')}")
except Exception as e:
    print(f"Failed: {e}")
