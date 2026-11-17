import json
import os

with open('showcase/public/all_avatars_data.json', 'r', encoding='utf-8') as f:
    avatars = json.load(f)

print(f"Loaded {len(avatars)} avatars.")

# Check models.json for authoritative 3D models list
with open('showcase/public/models/models.json', 'r', encoding='utf-8') as f:
    models_data = json.load(f)

models_list = models_data if isinstance(models_data, list) else models_data.get('models', [])
print(f"Loaded {len(models_list)} authoritative 3D models from models.json.")

# Check thumbnails
thumb_dir = 'showcase/public/thumbnails'
available_thumbs = os.listdir(thumb_dir) if os.path.exists(thumb_dir) else []
print(f"Available extracted thumbnails: {available_thumbs}")

# Map each 3D model to its thumbnail
model_to_thumb = {
    'model.avatar_sample_a': './public/thumbnails/AvatarSample_A.png',
    'model.avatar_sample_b': './public/thumbnails/AvatarSample_B.png',
    'model.avatar_sample_c': './public/thumbnails/AvatarSample_C.png',
    'model.sendagaya_shibu': './public/thumbnails/Sendagaya_Shibu.png',
    'model.fem_vroid': './public/thumbnails/fem_vroid.png',
    'model.masc_vroid': './public/thumbnails/masc_vroid.png',
    'model.avatar_orion': './public/thumbnails/Avatar_Orion.svg',
    'model.alicia_solid': './public/thumbnails/AliciaSolid.png',
    'model.darkness_shibu': './public/thumbnails/Darkness_Shibu.png',
    'model.sakurada_fumiriya': './public/thumbnails/Sakurada_Fumiriya.png',
    'model.seed_san': './public/thumbnails/Seed-san.png',
    'model.sendagaya_shino': './public/thumbnails/Sendagaya_Shino.png',
    'model.victoria_rubin': './public/thumbnails/Victoria_Rubin.png',
    'model.vita': './public/thumbnails/Vita.png',
    'model.vivi': './public/thumbnails/Vivi.png',
    'model.monero_chan': './public/thumbnails/monero-chan.png',
    'model.readyplayer_me': './public/thumbnails/readyplayer.me.svg',
    'model.soldier': './public/thumbnails/Soldier.svg',
    'model.celestia': './public/thumbnails/celestia.svg',
    'model.paladin': './public/thumbnails/paladin.svg',
    'model.kira': './public/thumbnails/kira.svg',
    'model.robot_expressive': './public/thumbnails/RobotExpressive.svg',
}

# Update 3D models in avatar list
confirmed_ids = {
    'model.darkness_shibu', 'model.victoria_rubin', 'model.sendagaya_shino',
    'model.alicia_solid', 'model.seed_san', 'model.readyplayer_me',
    'model.soldier', 'model.monero_chan', 'model.sakurada_fumiriya',
    'model.vivi', 'model.vita', 'model.avatar_sample_b', 'model.avatar_sample_a'
}

needs_adjust_ids = {
    'model.robot_expressive', 'model.kira', 'model.paladin', 'model.celestia'
}

new_vrm_ids = {
    'model.avatar_sample_c', 'model.sendagaya_shibu', 'model.fem_vroid',
    'model.masc_vroid', 'model.avatar_orion'
}

# Mapping 2D PetPacks to authentic 3D humanoid rigs
petpack_3d_mapping = {
    'anya': {'model': './public/models/AvatarSample_A.vrm', 'name': 'Anime Girl Rig (Annie)', 'bones': 206},
    'frieren': {'model': './public/models/Victoria_Rubin.vrm', 'name': 'Elven Sorceress Rig', 'bones': 137},
    'nezuko': {'model': './public/models/fem_vroid.vrm', 'name': 'Anime Heroine Rig (Sakura)', 'bones': 59},
    'power': {'model': './public/models/Sakurada_Fumiriya.vrm', 'name': 'Anime Fiend Rig', 'bones': 137},
    'mikasa': {'model': './public/models/Darkness_Shibu.vrm', 'name': 'Anime Scout Rig', 'bones': 137},
    'yor': {'model': './public/models/Victoria_Rubin.vrm', 'name': 'Assassin Heroine Rig', 'bones': 137},
    'rukia': {'model': './public/models/Sendagaya_Shino.vrm', 'name': 'Soul Reaper Heroine Rig', 'bones': 129},
    'nami': {'model': './public/models/Vivi.vrm', 'name': 'Navigator Rig (Vivi)', 'bones': 129},
    'gojo': {'model': './public/models/Sendagaya_Shibu.vrm', 'name': 'Special Grade Sorcerer Rig', 'bones': 129},
    'goku': {'model': './public/models/masc_vroid.vrm', 'name': 'Martial Arts Hero Rig (Ren)', 'bones': 59},
    'naruto': {'model': './public/models/AvatarSample_B.vrm', 'name': 'Ninja Hero Rig', 'bones': 129},
    'sasuke': {'model': './public/models/Sendagaya_Shibu.vrm', 'name': 'Rogue Ninja Rig', 'bones': 129},
    'tanjiro': {'model': './public/models/masc_vroid.vrm', 'name': 'Demon Slayer Rig (Ren)', 'bones': 59},
    'denji': {'model': './public/models/AvatarSample_B.vrm', 'name': 'Devil Hunter Rig', 'bones': 129},
    'levi': {'model': './public/models/AvatarSample_C.vrm', 'name': 'Captain Rig', 'bones': 101},
    'luffy': {'model': './public/models/masc_vroid.vrm', 'name': 'Pirate Captain Rig', 'bones': 59},
    'zoro': {'model': './public/models/masc_vroid.vrm', 'name': 'Swordsman Rig', 'bones': 59},
    'saitama': {'model': './public/models/masc_vroid.vrm', 'name': 'Hero Rig', 'bones': 59},
    'yuji': {'model': './public/models/AvatarSample_B.vrm', 'name': 'Jujutsu Vessel Rig', 'bones': 129},
    'sanji': {'model': './public/models/Sendagaya_Shibu.vrm', 'name': 'Cook Rig', 'bones': 129},
    'shiva': {'model': './public/models/Avatar_Orion.vrm', 'name': 'Cosmic Deity Rig (Orion)', 'bones': 65},
    'krishna': {'model': './public/models/masc_vroid.vrm', 'name': 'Divine Charioteer Rig', 'bones': 59},
    'ganesha': {'model': './public/models/paladin.glb', 'name': 'Guardian Deity Rig', 'bones': 65},
    'hanuman': {'model': './public/models/Avatar_Orion.vrm', 'name': 'Devoted Deity Rig', 'bones': 65},
    'rama': {'model': './public/models/Avatar_Orion.vrm', 'name': 'Divine King Rig', 'bones': 65},
    'lakshmi': {'model': './public/models/Victoria_Rubin.vrm', 'name': 'Goddess of Prosperity Rig', 'bones': 137},
    'saraswati': {'model': './public/models/fem_vroid.vrm', 'name': 'Goddess of Arts Rig', 'bones': 59},
    'sita': {'model': './public/models/Vita.vrm', 'name': 'Divine Empress Rig', 'bones': 129},
    'radha': {'model': './public/models/Vivi.vrm', 'name': 'Divine Beloved Rig', 'bones': 129},
    'sun-wukong': {'model': './public/models/Avatar_Orion.vrm', 'name': 'Monkey King Rig', 'bones': 65},
    'kodama-spirit': {'model': './public/models/Seed-san.vrm', 'name': 'Forest Spirit Rig', 'bones': 101},
    'starter-aoi': {'model': './public/models/AliciaSolid.vrm', 'name': 'Coder Mascot Rig (Alicia)', 'bones': 150},
    'witch-apprentice': {'model': './public/models/Vita.vrm', 'name': 'Witch Apprentice Rig', 'bones': 129},
    'woodland-guardian': {'model': './public/models/paladin.glb', 'name': 'Woodland Guardian Rig', 'bones': 65},
    'valley-flyer': {'model': './public/models/Seed-san.vrm', 'name': 'Sky Flyer Rig', 'bones': 101},
    'killua': {'model': './public/models/AvatarSample_B.vrm', 'name': 'Lightning Assassin Rig', 'bones': 129},
    'gon': {'model': './public/models/masc_vroid.vrm', 'name': 'Hunter Boy Rig', 'bones': 59},
    'ichigo': {'model': './public/models/Sendagaya_Shibu.vrm', 'name': 'Shinigami Hero Rig', 'bones': 129},
    'pikachu': {'model': './public/models/Seed-san.vrm', 'name': 'Electric Familiar Rig', 'bones': 101},
    'momiji': {'model': './public/models/Sakurada_Fumiriya.vrm', 'name': 'Fox Maiden Rig', 'bones': 137},
    'arjuna': {'model': './public/models/Avatar_Orion.vrm', 'name': 'Archer Hero Rig', 'bones': 65},
    'genos': {'model': './public/models/RobotExpressive.glb', 'name': 'Cyborg Hero Rig', 'bones': 65},
    'light': {'model': './public/models/AvatarSample_B.vrm', 'name': 'Scholar Rig', 'bones': 129},
    'l-lawliet': {'model': './public/models/Sendagaya_Shibu.vrm', 'name': 'Detective Rig', 'bones': 129},
}

for av in avatars:
    av_id = av.get('id', '')
    
    # 1. 3D Humanoid Models
    if av.get('avatar_type') in ['3d_humanoid_model', '3d_model']:
        av['avatar_type'] = '3d_humanoid_model'
        av['has_3d_model'] = True
        av['preview_type'] = '3d_portrait'
        if av_id in model_to_thumb:
            av['image_url'] = model_to_thumb[av_id]
            av['thumbnail_url'] = model_to_thumb[av_id]
        
        if av_id in confirmed_ids:
            av['review_status'] = 'confirmed'
            av['review_badge'] = '💖 Confirmed for Miyora'
            av['priority_order'] = 1
        elif av_id in new_vrm_ids:
            av['review_status'] = 'new_candidate'
            av['review_badge'] = '✨ New Authentic VRM'
            av['priority_order'] = 2
        elif av_id in needs_adjust_ids:
            av['review_status'] = 'adjustment_needed'
            av['review_badge'] = '⚠️ Needs Adjustment'
            av['priority_order'] = 3
        else:
            av['priority_order'] = 4

    # 2. Bundled 2D PetPacks - mapped to authentic 3D humanoid rigs
    elif av.get('avatar_type') in ['bundled_2d_petpack', 'pet_pack', 'pet_pack_3d']:
        av['avatar_type'] = 'bundled_2d_petpack'
        slug = av.get('slug', av_id.replace('agentpet.', ''))
        mapping = petpack_3d_mapping.get(slug)
        if mapping:
            av['has_3d_model'] = True
            av['url'] = mapping['model']
            av['rig_name'] = mapping['name']
            av['bones'] = mapping['bones']
            av['ik_compatible'] = True
            av['format'] = '2D Sprite + 3D Humanoid Rig'
            av['review_badge'] = '✨ 3D Humanoid Ready'
        else:
            av['has_3d_model'] = True
            av['url'] = './public/models/AvatarSample_A.vrm'
            av['rig_name'] = 'Standard Humanoid Rig'
            av['bones'] = 206
            av['ik_compatible'] = True
            av['format'] = '2D Sprite + 3D Humanoid Rig'
            av['review_badge'] = '✨ 3D Humanoid Ready'
        av['priority_order'] = 5

    # 3. Catalog Specs - 3D Humanoid Rig
    else:
        av['avatar_type'] = 'catalog_spec'
        av['has_3d_model'] = True
        # Map body to appropriate authentic base
        if av.get('body') == 'beast':
            av['url'] = './public/models/Seed-san.vrm'
            av['rig_name'] = 'Beast / Familiar 3D Rig'
        elif av.get('category') in ['deity', 'deity_inspired', 'mythological']:
            av['url'] = './public/models/Avatar_Orion.vrm'
            av['rig_name'] = 'Deity / Celestial 3D Rig'
        else:
            av['url'] = './public/models/fem_vroid.vrm'
            av['rig_name'] = 'Anime Humanoid 3D Rig'
        av['bones'] = 59
        av['ik_compatible'] = True
        av['priority_order'] = 6

with open('showcase/public/all_avatars_data.json', 'w', encoding='utf-8') as f:
    json.dump(avatars, f, indent=2, ensure_ascii=False)

print("Successfully updated showcase/public/all_avatars_data.json!")
