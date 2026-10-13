"""Tests staggering, unit conversion and unsupported-height masks."""
from prepare_tiegcm import height_for, on_heights, factor
import xarray as xr
import numpy as np
zg=np.array([100,200,300.])[:,None,None]*1e5
zg=np.broadcast_to(zg,(3,2,2)).copy()
ds=xr.Dataset({'ZG':(('ilev','lat','lon'),zg)},coords={'ilev':[0.,1.,2.],'lev':[.5,1.5,2.5],'lat':[0,5],'lon':[0,5]})
ds.ZG.attrs['units']='cm'
h=height_for(ds,'lev').transpose('lev','lat','lon').values
assert np.allclose(h[:2,0,0],[150,250]) and np.isnan(h[2,0,0])
v=np.array([1,2,3.])[:,None,None]*np.ones((3,2,2))
out=on_heights(v, zg*1e-5, [50,150,350])
assert np.isnan(out[[0,2]]).all() and np.allclose(out[1],1.5)
assert factor('cm-3','density')==1e6 and factor('cm/s','wind')==.01
try: factor('unknown','height')
except ValueError: pass
else: raise AssertionError('Unknown units silently accepted')
print('TIE-GCM staggering, units, height interpolation and missing-coverage checks passed')
