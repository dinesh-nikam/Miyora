import struct
import json
import os
import sys

def parse_glb_or_vrm(file_path):
    with open(file_path, 'rb') as f:
        magic = f.read(4)
        if magic != b'glTF':
            raise ValueError(f"Not a valid glTF/GLB/VRM file: magic is {magic}")
        version, length = struct.unpack('<II', f.read(8))
        chunk_len, chunk_type = struct.unpack('<II', f.read(8))
        if chunk_type != 0x4E4F534A: # 'JSON'
            raise ValueError("Expected JSON chunk first")
        json_bytes = f.read(chunk_len)
        gltf = json.loads(json_bytes.decode('utf-8'))

    # Extract metadata
    meta = {}
    title = os.path.basename(file_path)
    author = "Unknown"
    license_str = "Unknown"
    
    # Check VRM extensions
    extensions = gltf.get('extensions', {})
    vrm0 = extensions.get('VRM', {})
    vrm1 = extensions.get('VRMC_vrm', {})
    
    is_vrm = False
    vrm_human_bones = {}
    
    if vrm0:
        is_vrm = True
        vrm_meta = vrm0.get('meta', {})
        title = vrm_meta.get('title', title)
        author = vrm_meta.get('author', author)
        license_str = vrm_meta.get('licenseName', license_str)
        human_bones = vrm0.get('humanoid', {}).get('humanBones', [])
        for hb in human_bones:
            vrm_human_bones[hb.get('bone')] = hb.get('node')
    elif vrm1:
        is_vrm = True
        vrm_meta = vrm1.get('meta', {})
        title = vrm_meta.get('name', title)
        authors = vrm_meta.get('authors', [])
        author = ", ".join(authors) if isinstance(authors, list) else str(authors)
        license_str = vrm_meta.get('licenseUrl', license_str)
        human_bones = vrm1.get('humanoid', {}).get('humanBones', {})
        for b_name, b_info in human_bones.items():
            vrm_human_bones[b_name] = b_info.get('node')

    # Calculate bone count
    nodes = gltf.get('nodes', [])
    skins = gltf.get('skins', [])
    skin_joints = set()
    for s in skins:
        for j in s.get('joints', []):
            skin_joints.add(j)
    
    bone_count = len(skin_joints) if skin_joints else len(vrm_human_bones)
    
    # Check standard humanoid bones
    standard_bones = [
        'hips', 'spine', 'chest', 'neck', 'head',
        'leftUpperArm', 'leftLowerArm', 'leftHand',
        'rightUpperArm', 'rightLowerArm', 'rightHand',
        'leftUpperLeg', 'leftLowerLeg', 'leftFoot',
        'rightUpperLeg', 'rightLowerLeg', 'rightFoot'
    ]
    
    # Check IK compatibility
    ik_compatible = False
    if is_vrm and len(vrm_human_bones) >= 15:
        ik_compatible = True
    elif skins:
        # Check node names for standard mixamo / humanoid bones
        node_names = [nodes[j].get('name', '').lower() for j in skin_joints if j < len(nodes)]
        has_hips = any('hips' in n or 'pelvis' in n for n in node_names)
        has_spine = any('spine' in n for n in node_names)
        has_arm = any('arm' in n or 'shoulder' in n for n in node_names)
        has_leg = any('leg' in n or 'thigh' in n or 'foot' in n for n in node_names)
        if has_hips and has_spine and has_arm and has_leg:
            ik_compatible = True

    # Calculate polygon count
    # Each mesh primitive has accessors
    accessors = gltf.get('accessors', [])
    poly_count = 0
    for mesh in gltf.get('meshes', []):
        for prim in mesh.get('primitives', []):
            indices_idx = prim.get('indices')
            if indices_idx is not None and indices_idx < len(accessors):
                count = accessors[indices_idx].get('count', 0)
                poly_count += count // 3
            elif 'POSITION' in prim.get('attributes', {}):
                pos_idx = prim['attributes']['POSITION']
                if pos_idx < len(accessors):
                    count = accessors[pos_idx].get('count', 0)
                    poly_count += count // 3

    return {
        'title': title,
        'author': author,
        'license': license_str,
        'is_vrm': is_vrm,
        'bone_count': bone_count,
        'polygon_count': poly_count,
        'ik_compatible': ik_compatible,
        'vrm_bones': list(vrm_human_bones.keys()) if is_vrm else [],
        'skins_count': len(skins),
        'animations_count': len(gltf.get('animations', []))
    }

if __name__ == '__main__':
    if len(sys.argv) > 1:
        res = parse_glb_or_vrm(sys.argv[1])
        print(json.dumps(res, indent=2))
