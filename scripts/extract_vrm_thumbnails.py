import json
import struct
import os

os.makedirs('showcase/public/thumbnails', exist_ok=True)

def extract_vrm_thumbnail(vrm_path, out_name):
    out_path = os.path.join('showcase/public/thumbnails', out_name)
    if os.path.exists(out_path):
        return out_path
    
    with open(vrm_path, 'rb') as f:
        magic = f.read(4)
        if magic != b'glTF':
            return None
        version, length = struct.unpack('<II', f.read(8))
        json_len, json_type = struct.unpack('<II', f.read(8))
        if json_type != 0x4E4F534A:
            return None
        json_bytes = f.read(json_len)
        data = json.loads(json_bytes.decode('utf-8'))
        
        # Check VRM meta texture
        vrm_meta = data.get('extensions', {}).get('VRM', {}).get('meta', {})
        texture_idx = vrm_meta.get('texture')
        if texture_idx is None:
            # Fallback to VRMC_vrm
            vrmc_meta = data.get('extensions', {}).get('VRMC_vrm', {}).get('meta', {})
            texture_idx = vrmc_meta.get('thumbnailImage')
            
        if texture_idx is None:
            print(f"No meta texture for {vrm_path}")
            return None
            
        textures = data.get('textures', [])
        images = data.get('images', [])
        buffer_views = data.get('bufferViews', [])
        
        image_idx = texture_idx
        if texture_idx < len(textures):
            image_idx = textures[texture_idx].get('source', texture_idx)
            
        if image_idx >= len(images):
            print(f"Image idx {image_idx} out of range in {vrm_path}")
            return None
            
        image_info = images[image_idx]
        bv_idx = image_info.get('bufferView')
        if bv_idx is None or bv_idx >= len(buffer_views):
            print(f"No bufferView for image {image_idx} in {vrm_path}")
            return None
            
        bv = buffer_views[bv_idx]
        offset = bv.get('byteOffset', 0)
        byte_len = bv.get('byteLength', 0)
        
        # Binary chunk header
        f.seek(12 + 8 + json_len)
        bin_len, bin_type = struct.unpack('<II', f.read(8))
        
        f.seek(12 + 8 + json_len + 8 + offset)
        img_bytes = f.read(byte_len)
        
        with open(out_path, 'wb') as out_f:
            out_f.write(img_bytes)
        print(f"Extracted thumbnail for {vrm_path} -> {out_path} ({len(img_bytes)} bytes)")
        return out_path

models_dir = 'showcase/public/models'
for fname in sorted(os.listdir(models_dir)):
    if fname.lower().endswith('.vrm'):
        base = os.path.splitext(fname)[0]
        vpath = os.path.join(models_dir, fname)
        try:
            extract_vrm_thumbnail(vpath, f"{base}.png")
        except Exception as e:
            print(f"Error {fname}: {e}")
