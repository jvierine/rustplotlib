"""Extract binary render transport from the gallery's authoritative HDF5 data."""
from pathlib import Path
import json
import h5py
import numpy as np
root=Path(__file__).resolve().parents[1]
out=root/'output/essays'
out.mkdir(parents=True,exist_ok=True)
with h5py.File(root/'gallery/khi.h5') as f:
    data=f['dye'][:]
    data.astype('<f4').tofile(out/'khi.f32')
    meta={k:f['diagnostics/'+k][:].tolist() for k in f['diagnostics']}
    (out/'khi.json').write_text(json.dumps(meta))
with h5py.File(root/'gallery/dimant_oppenheim.h5') as f:
    f['display_density'][:].astype('<f4').tofile(out/'meteor.f32')
print('Prepared visual essay transport from HDF5')
