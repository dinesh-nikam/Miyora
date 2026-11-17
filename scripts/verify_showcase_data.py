import json
import os

data = json.load(open('showcase/public/all_avatars_data.json', encoding='utf-8'))

def sort_key(a):
    is_conf = 0 if a.get('review_status') == 'confirmed' else 1
    prio = a.get('priority_order', 99)
    name = a.get('name', '')
    return (is_conf, prio, name)

sorted_data = sorted(data, key=sort_key)
print('--- Top 20 Avatars on Page Load ---')
all_ok = True
for idx, a in enumerate(sorted_data[:20]):
    img = a.get('image_url') or a.get('thumbnail_url', '')
    url = a.get('url', '')
    img_clean = img.replace('./', 'showcase/').replace('/', os.sep) if img else ''
    url_clean = url.replace('./', 'showcase/').replace('/', os.sep) if url else ''
    img_exists = os.path.exists(img_clean) if img_clean else False
    url_exists = os.path.exists(url_clean) if url_clean else False
    status = a.get('review_status')
    name = a.get('name')
    aid = a.get('id')
    print(f"{idx+1}. {name} ({aid}) | status={status} | img_ok={img_exists} | 3d_ok={url_exists}")
    if not url_exists:
        all_ok = False

print('Verification result: 3D models verified:', all_ok)
