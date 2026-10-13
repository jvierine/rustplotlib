"""Freeze three small GAMERA J-path examples from the user's currents viewer."""
from pathlib import Path
import json,hashlib
import numpy as np
import h5py
source=Path('/Users/jvi019/src/currents/gcurrents')
out=Path('gallery/currents');out.mkdir(parents=True,exist_ok=True)
meta=json.loads((source/'snapshots.json').read_text())
selected=[]
with h5py.File(out/'paths.h5','w') as h:
    h.attrs['source']='https://juha.no/gcurrents/'
    h.attrs['code_source']='jvierine/currents/gcurrents; GAMERA J paths, not magnetic field lines'
    for index in [0,4,9]:
        row=meta[index];record=json.loads((source/row['file']).read_text());count={};paths=[]
        for path in record['paths']:
            group=path['group'];count[group]=count.get(group,0)+1
            if count[group]<=8:paths.append(path)
        record={k:record[k] for k in ['hours','utc','model_time_s']};record['paths']=paths
        name=f'{index}.json';(out/name).write_text(json.dumps(record,separators=(',',':')))
        selected.append({'file':name,'utc':record['utc'],'hours':record['hours']})
        g=h.create_group(str(index));g.attrs['source_sha256']=hashlib.sha256((source/row['file']).read_bytes()).hexdigest()
        g.attrs['utc']=record['utc']
        for i,p in enumerate(paths):
            d=g.create_dataset(f'path_{i}',data=np.asarray(p['points'],dtype='f4'),compression='gzip');d.attrs['group']=p['group'];d.attrs['units']='Earth radii'
    h.attrs['script']='scripts/prepare_currents.py'
(out/'manifest.json').write_text(json.dumps(selected,indent=2))
print('Exported current paths; no synthetic connectors')
