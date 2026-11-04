"""Audit real exported files and write the versioned, relative-path model registry."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--catalog',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--tools',type=Path,required=True)
    a=p.parse_args()
    specs=json.loads(a.catalog.read_text(encoding='utf-8'))['characters']
    entries=[];missing=[]
    for spec in specs:
        slug=spec['id'].split('.')[-1];folder=a.output/slug
        if not (folder/'production.json').exists():
            missing.append(spec['id']);continue
        report=json.loads((folder/'model.validation.json').read_text())
        model=folder/'model.vrm';source=folder/'source.blend'
        digest=hashlib.sha256(model.read_bytes()).hexdigest()
        if report['structuralValidation']!='passed' or digest!=report['sha256'] or source.stat().st_size<1000:
            raise ValueError('Invalid completed asset '+spec['id'])
        entries.append({'id':spec['id'],'name':spec['name'],'body':spec['body'],'model':slug+'/model.vrm','source':slug+'/source.blend','sha256':digest,'triangles':report['triangleCount'],'animations':report['animationNames']})
    status={'schemaVersion':1,'requested':len(specs),'structurallyValidated':len(entries),'missing':missing,'bodyCounts':dict(Counter(e['body'] for e in entries)),'runtimeValidated':0,'artReview':'first-pass-models-require-individual-art-review','style':'full-anime','paidServicesUsed':False}
    (a.output/'production-status.json').write_text(json.dumps(status,indent=2),encoding='utf-8')
    print(json.dumps(status))
    if missing:
        return 1
    (a.output/'collection.json').write_text(json.dumps({'schemaVersion':1,'style':'full-anime','characters':entries},indent=2),encoding='utf-8')
    tools=a.output/'authoring';tools.mkdir(exist_ok=True)
    for name in ('author_characters.py','catalog_author.py','package_vrm.py','collect_catalog.py'):
        shutil.copy2(a.tools/name,tools/name)
    return 0


if __name__=='__main__':
    raise SystemExit(main())
