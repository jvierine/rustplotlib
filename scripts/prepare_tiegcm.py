"""Convert a TIE-GCM secondary history into height-resolved HDF5 fields.

conda run -n base python scripts/prepare_tiegcm.py history.nc --output model.h5
Geometric heights follow NCAR TIE-GCM's lev/ilev staggering, not level index.
"""
import argparse
import hashlib
import json
from pathlib import Path
import numpy as np
import xarray as xr
import h5py

REFERENCE = 'https://www.hao.ucar.edu/modeling/tgcm/tiegcm2.0/userguide/html/grid.html'

def factor(units, kind):
    u = units.lower().replace(' ', '').replace('**', '^')
    choices = {'height': {'cm': 1e-5, 'm': 1e-3, 'km': 1.},
               'wind': {'cm/s': .01, 'cms-1': .01, 'cm/s ': .01, 'm/s': 1., 'ms-1': 1.},
               'density': {'cm-3': 1e6, 'cm^-3': 1e6, '1/cm3': 1e6, 'm-3': 1., 'm^-3': 1.},
               'temperature': {'k': 1., 'kelvin': 1.}}
    if u not in choices[kind]:
        raise ValueError(f'Unsupported {kind} units: {units!r}; refusing implicit conversion')
    return choices[kind][u]

def height_for(ds, vertical):
    if vertical == 'ilev':
        return ds['ZG'] * factor(ds['ZG'].attrs.get('units', ''), 'height')
    if 'ZGMID' in ds:
        return ds['ZGMID'] * factor(ds['ZGMID'].attrs.get('units', ''), 'height')
    # Linear interpolation in log-pressure coordinates; do not extrapolate above
    # the last interface. TIE-GCM's top midpoint is often above the last ZG.
    zg = ds['ZG'] * factor(ds['ZG'].attrs.get('units', ''), 'height')
    return zg.interp(ilev=ds['lev'].values).rename({'ilev': 'lev'})

def on_heights(values, heights, targets):
    nz, nlat, nlon = values.shape
    out = np.full((len(targets), nlat, nlon), np.nan, dtype=np.float32)
    for j in range(nlat):
        for i in range(nlon):
            h = heights[:, j, i]; v = values[:, j, i]
            valid = np.isfinite(h) & np.isfinite(v)
            h, v = h[valid], v[valid]
            if len(h) < 2: continue
            if np.any(np.diff(h) <= 0):
                raise ValueError('Non-monotone geometric heights')
            out[:, j, i] = np.interp(targets, h, v, left=np.nan, right=np.nan)
    return out

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('input', type=Path); p.add_argument('--output', type=Path, required=True)
    p.add_argument('--time-index', type=int, default=0)
    p.add_argument('--altitudes', nargs=3, type=float, default=[100, 500, 5], metavar=('MIN', 'MAX', 'STEP'))
    args = p.parse_args()
    ds = xr.open_dataset(args.input)
    if 'time' in ds.dims: ds = ds.isel(time=args.time_index)
    targets = np.arange(args.altitudes[0], args.altitudes[1]+.1*args.altitudes[2], args.altitudes[2])
    if len(targets) < 2 or np.any(np.diff(targets) <= 0): raise ValueError('Invalid altitude range')
    fields = [('TN', 'temperature', 'K'), ('NE', 'density', 'm^-3'),
              ('UN', 'wind', 'm s^-1'), ('VN', 'wind', 'm s^-1')]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with h5py.File(args.output, 'w') as out:
        out.attrs['source_path'] = str(args.input)
        out.attrs['source_sha256'] = hashlib.sha256(args.input.read_bytes()).hexdigest()
        out.attrs['reference'] = REFERENCE
        out.attrs['script'] = 'scripts/prepare_tiegcm.py'
        out.attrs['time_index'] = args.time_index
        out.attrs['height_interpolation'] = 'columnwise linear; no extrapolation'
        for name, data, units in [('altitude_km', targets, 'km'), ('latitude_deg', ds.lat.values, 'degrees_north'), ('longitude_deg', ds.lon.values, 'degrees_east')]:
            d = out.create_dataset('coordinates/'+name, data=data); d.attrs['units'] = units
        for name, kind, units in fields:
            if name not in ds: continue
            field = ds[name]
            vertical = 'ilev' if 'ilev' in field.dims else 'lev'
            h = height_for(ds, vertical).transpose(vertical, 'lat', 'lon').values
            v = field.transpose(vertical, 'lat', 'lon').values * factor(field.attrs.get('units', ''), kind)
            data = on_heights(v, h, targets)
            d = out.create_dataset('fields/'+name, data=data, compression='gzip', shuffle=True)
            d.attrs['units'] = units; d.attrs['original_vertical_coordinate'] = vertical
            d.attrs['height_source'] = 'ZG' if vertical == 'ilev' else 'ZGMID' if 'ZGMID' in ds else 'ZG interpolated in log pressure'
            out.create_dataset('valid/'+name, data=np.isfinite(data), compression='gzip')
        if not len(out.get('fields', [])): raise ValueError('No supported fields present')
    print(args.output)
if __name__ == '__main__': main()
