"""Keep numerical gallery data and checks in HDF5; binary files are browser transport."""
import json,hashlib
from pathlib import Path
import h5py
import numpy as np
root=Path(__file__).resolve().parents[1]
with h5py.File(root/'gallery/khi.h5','w') as f:
    meta=json.loads((root/'output/khi/diagnostics.json').read_text());s=meta['samples']
    data=np.fromfile(root/'output/khi/dye.f32',dtype='<f4').reshape(len(s),128,128)
    assert np.isfinite(data).all()
    mean=np.array([v['mean_dye'] for v in s]);assert np.ptp(mean)<1e-12
    for key in ['energy','enstrophy']:
        values=np.array([v[key] for v in s]);assert np.all(np.diff(values)<1e-8),key
    assert meta['fourier_decay_error']<1e-10
    f['dye']=data
    for key in s[0]:f['diagnostics/'+key]=[v[key] for v in s]
    f.attrs['configuration_json']=json.dumps(meta)
    f.attrs['script']='examples/khi.rs'
    f.attrs['display']='Bilinear interpolation at cell centres; physical arrays retained'
with h5py.File(root/'gallery/heimdall/data.h5','w') as f:
    f.attrs['provenance_json']=(root/'gallery/heimdall/provenance.json').read_text()
    for path in (root/'gallery/heimdall').glob('*.bin'):
        f.create_dataset(path.stem,data=np.frombuffer(path.read_bytes(),dtype='u1'),compression='gzip')
        f[path.stem].attrs['format']='HEIMDALL little-endian float16 browser product'
with h5py.File(root/'gallery/dimant_oppenheim.h5','w') as f:
    f['z_over_lambda_T']=np.linspace(-5,20,1001)
    f['r_over_lambda_T']=np.linspace(-10,10,801)
    f.create_dataset('display_density',data=np.fromfile(root/'output/dimant/density.f32',dtype='<f4').reshape(800,1000),compression='gzip')
    f.attrs['normalization']='8*pi*ne/n_star; ne=ni'
    f.attrs['source']='https://github.com/jvierine/sabmod/blob/bb2388d4f45dba6aaa0e944f29bdf686e54a30d4/dimant_oppenheim.py'
    f.attrs['reference']='https://doi.org/10.1002/2017JA023963'
    f.attrs['script']='examples/dimant_oppenheim.rs; examples/support/dimant.rs'
    checks=json.loads((root/'output/dimant/checks.json').read_text())
    for q in checks:
        assert np.isfinite(q['rust']) and np.isfinite(q['refined'])
        assert abs(q['rust']-q['refined'])/max(abs(q['refined']),1e-12)<1e-8
    f.attrs['quadrature_checks_json']=json.dumps(checks)
    f['color_table_rgb']=np.frombuffer((root/'examples/support/dimant_colormap.rgb').read_bytes(),dtype='u1').reshape(-1,3)
    f.attrs['color_table_provenance_json']=(root/'examples/support/dimant_colormap.json').read_text()
print('KHI invariants and diffusion checks passed; HDF5 examples archived')
