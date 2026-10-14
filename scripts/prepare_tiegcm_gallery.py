"""Render transport for paired TIE-GCM temperatures at common geometric heights."""
from pathlib import Path
import h5py,numpy as np,json
from scipy.interpolate import RegularGridInterpolator
out=Path('output/tiegcm');out.mkdir(parents=True,exist_ok=True)
with h5py.File('gallery/tiegcm/section.h5') as h:
    d=h['distance_km'][:];z=h['altitude_km'][:];t=h['time_s'][:]
    x=np.linspace(d[0],d[-1],261);zz,xx=np.meshgrid(z,x,indexing='ij')
    for frame in range(len(t)):
        vals=RegularGridInterpolator((z,d),h['delta_temperature_K'][frame],bounds_error=False,fill_value=np.nan)(np.stack([zz,xx],axis=-1))
        vals.astype('<f4').tofile(out/f'{frame}.f32')
    meta={'distance':x.tolist(),'altitude':z.tolist(),'time_s':t.tolist(),'frames':[6,12,24,48]}
    (out/'geometry.json').write_text(json.dumps(meta))
print('TIE-GCM display uses bilinear interpolation; missing geometric-height coverage preserved')
