import json

with open('docs/detailed_character_specifications.json', 'r', encoding='utf-8') as f:
    detailed_chars = json.load(f)

with open('showcase/public/all_avatars_data.json', 'r', encoding='utf-8') as f:
    all_avatars = json.load(f)

# Index detailed characters by ID and slug
detailed_map = {}
for c in detailed_chars:
    detailed_map[c['id']] = c
    if c.get('slug'):
        detailed_map[c['slug']] = c
        detailed_map[f"agentpet.{c['slug']}"] = c
        detailed_map[f"model.{c['slug']}"] = c

updated_count = 0
for av in all_avatars:
    av_id = av.get('id', '')
    slug = av.get('slug', '')
    match = detailed_map.get(av_id) or detailed_map.get(slug)
    if match:
        av['detailed_spec'] = match
        av['canonical_stats'] = match.get('canonical_stats')
        av['micro_physical_details'] = match.get('micro_physical_details')
        av['wardrobe_and_layering'] = match.get('wardrobe_and_layering')
        av['anatomical_3d_rig'] = match.get('anatomical_3d_rig')
        av['miyora_companion_role'] = match.get('miyora_companion_role')
        av['native_name'] = match.get('native_name')
        av['origin_lore'] = match.get('origin_lore')
        if match.get('micro_physical_details', {}).get('skin_tone'):
            av.setdefault('palette', {})['skin'] = match['micro_physical_details']['skin_tone']
        if match.get('micro_physical_details', {}).get('hair', {}).get('base_color'):
            av.setdefault('palette', {})['hair'] = match['micro_physical_details']['hair']['base_color']
        if match.get('micro_physical_details', {}).get('eyes', {}).get('color_base'):
            av.setdefault('palette', {})['eyes'] = match['micro_physical_details']['eyes']['color_base']
        updated_count += 1

with open('showcase/public/all_avatars_data.json', 'w', encoding='utf-8') as f:
    json.dump(all_avatars, f, indent=2, ensure_ascii=False)

print(f"Merged deep specs into showcase/public/all_avatars_data.json: {updated_count} avatars enriched with micro-details!")
