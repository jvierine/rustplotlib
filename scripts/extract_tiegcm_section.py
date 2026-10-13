"""Read a completed control/pulse TIE-GCM pair into a compact geometric-height cut.
Run in the existing TIE-GCM scientific environment on the simulation host.
"""
from pathlib import Path
import hashlib,json
import numpy as np
import netCDF4
root=Path('/home/j/tiegcm_runs/tromso_experiment')
paths=[root/'control/control_sech.nc',root/'perturbed/perturbed_sech.nc']
for log in [root/'control/control.log',root/'perturbed/perturbed.log']:
    assert 'NORMAL EXIT' in log.read_text()[-2000:],f'Not a completed run: {log}'
alt=np.arange(100.,502.5,2.5)
all_fields=[]
for path in paths:
    with netCDF4.Dataset(path) as ds:
        lon=ds['lon'][:];lat=ds['lat'][:];lonidx=int(np.argmin(abs(lon-20.)))
        latidx=np.flatnonzero((lat>=20)&(lat<=85))
        latcut=np.asarray(lat[latidx]);times=np.asarray(ds['time'][:]);lev=np.asarray(ds['lev'][:]);ilev=np.asarray(ds['ilev'][:])
        tn=np.ma.filled(ds['TN'][:,:,latidx,lonidx],np.nan).astype(float)
        if 'ZGMID' in ds.variables:
            heights=np.ma.filled(ds['ZGMID'][:,:,latidx,lonidx],np.nan).astype(float)/1e5
            height_source='ZGMID'
        else:
            zg=np.ma.filled(ds['ZG'][:,:,latidx,lonidx],np.nan).astype(float)/1e5
            heights=np.full_like(tn,np.nan)
            for t in range(len(times)):
                for j in range(len(latidx)):heights[t,:,j]=np.interp(lev,ilev,zg[t,:,j],left=np.nan,right=np.nan)
            height_source='ZG interpolated from ilev to lev in log pressure'
        assert getattr(ds['ZG'],'units','').lower()=='cm'
        out=np.full((len(times),len(alt),len(latidx)),np.nan,dtype='f4')
        for t in range(len(times)):
            for j in range(len(latidx)):
                h=heights[t,:,j];v=tn[t,:,j];valid=np.isfinite(h)&np.isfinite(v)
                if valid.sum()<2: continue
                assert np.all(np.diff(h[valid])>0)
                out[t,:,j]=np.interp(alt,h[valid],v[valid],left=np.nan,right=np.nan)
        assert np.isfinite(out[1:]).any(), "No geometric-height coverage"
        all_fields.append(out[:,:,::-1])
        time_units=ds['time'].units
        longitude=float(lon[lonidx])
# Latitude decreases along this equatorward meridian; values follow that order.
latcut=latcut[::-1]
distance=(69.65-latcut)*np.pi/180.*6371.
output=Path('/tmp/rustplotlib_tiegcm_section.h5')
with netCDF4.Dataset(output,'w',format='NETCDF4') as f:
    f.createDimension('time',len(times));f.createDimension('height',len(alt));f.createDimension('latitude',len(latcut))
    for name,dim,data in [('altitude_km','height',alt),('latitude_deg','latitude',latcut),('distance_km','latitude',distance),('time_s','time',times-times[0])]:
        f.createVariable(name,'f8',(dim,))[:]=data
    for name,data in [('control_temperature_K',all_fields[0]),('pulse_temperature_K',all_fields[1]),('delta_temperature_K',all_fields[1]-all_fields[0])]:
        f.createVariable(name,'f4',('time','height','latitude'),zlib=True)[:]=data
    f.setncattr('longitude_deg',longitude);f.setncattr('time_units',time_units);f.setncattr('height_source',height_source)
    f.setncattr('script','scripts/extract_tiegcm_section.py')
    f.setncattr('source_paths_json',json.dumps([str(p) for p in paths]))
    f.setncattr('source_hashes_json',json.dumps({str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}))
    f.setncattr('experiment','Paired quiet / controlled 30-minute Gaussian specific-heating pulse, Tromso; not assimilated Joule heating')
    f.setncattr('geometry','Spherical Earth R=6371 km, meridian cut; temperatures compared at common geometric heights')
print(output)
