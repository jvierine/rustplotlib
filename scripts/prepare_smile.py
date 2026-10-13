"""Map one released SMILE RGB frame using its existing star-tracked calibration."""
from pathlib import Path
import hashlib,json
import numpy as np
from PIL import Image
from scipy.ndimage import map_coordinates
import h5py
cal=json.loads(Path('output/smile/calibration.json').read_text())
source=Path('/Users/jvi019/src/heimdall-smile/viewers/smile/public/frames/025.png')
image=np.asarray(Image.open(source).convert('RGB'),dtype=float)/255.
nx,ny=1024,512
lon=(np.arange(nx)+.5)/nx*2*np.pi-np.pi
lat=(np.arange(ny)+.5)/ny*np.pi-np.pi/2
lon,lat=np.meshgrid(lon,lat)
unit=np.stack([np.cos(lat)*np.cos(lon),np.cos(lat)*np.sin(lon),np.sin(lat)],axis=-1)
axes=np.array(cal['shellAxes']);points=unit*axes
sight=points-np.array(cal['position_ecef_km'])
right,down,look=np.array(cal['axes'])
z=sight@look
normal=points/axes**2
visible=(normal*(np.array(cal['position_ecef_km'])-points)).sum(axis=-1)>0
x=sight@right/z;y=sight@down/z
p=np.pad(np.array(cal['optpar']), (0,max(0,12-len(cal['optpar']))))
assert cal['optmod']==20
r2=x*x+y*y;q=1+p[7]*r2+p[8]*r2*r2+p[9]*r2*r2*r2
u=p[0]*(x*q+2*p[10]*x*y+p[11]*(r2+2*x*x))+.5+p[5]-.5/cal['width']
v=p[1]*(y*q+p[10]*(r2+2*y*y)+2*p[11]*x*y)+.5+p[6]-.5/cal['height']
if cal.get('image_flip_x'):u=1-u
if cal.get('image_flip_y'):v=1-v
valid=visible&(z>0)&(u>=0)&(u<=1)&(v>=0)&(v<=1)
rgb=np.stack([map_coordinates(image[:,:,k],[v*cal['height']-.5,u*cal['width']-.5],order=1,mode='constant',cval=0) for k in range(3)],axis=-1)
valid&=~((rgb[:,:,0]>1.35*rgb[:,:,1])&(rgb[:,:,0]>1.2*rgb[:,:,2]))
signal=np.maximum(0,(rgb[:,:,0]+rgb[:,:,1])*.5-.25)
t=np.clip(signal/.22,0,1);alpha=t*t*(3-2*t)*valid
# Match the original Earth viewer: textured planet under the projected UVI.
# The full observed footprint is retained in mapped.h5; this is display composition.
earth_source=Path('/Users/jvi019/src/heimdall-smile/viewers/smile/public/data/earth.jpg')
earth=np.asarray(Image.open(earth_source).convert('RGB').resize((nx,ny),Image.Resampling.LANCZOS),dtype=float)[::-1]/255.
rgba=np.concatenate([rgb,alpha[:,:,None]],axis=-1)
rgba=np.uint8(np.clip(rgba,0,1)*255+.5)
out=Path('gallery/smile');out.mkdir(parents=True,exist_ok=True)
Image.fromarray(rgba).save(out/'mapped.png')
qd_source=Path('/Users/jvi019/src/heimdall-smile/viewers/smile/public/data/magnetic/qd-latitude-2026.bin')
qd=np.frombuffer(qd_source.read_bytes(),dtype=np.uint8).reshape(181,361).astype(float)-90.
qd=map_coordinates(qd,[(lat+np.pi/2)/np.pi*180.,(lon+np.pi)/(2*np.pi)*360.],order=1,mode='nearest')
edge=np.clip((qd-60.)/1.5,0,1)
display_alpha=.9*alpha*edge*edge*(3-2*edge)
composite=earth*(1-display_alpha[:,:,None])+rgb*display_alpha[:,:,None]
Image.fromarray(np.uint8(np.concatenate([composite,np.ones((ny,nx,1))],axis=-1)*255+.5)).save(out/'earth_uvi.png')
provenance_earth_hash=hashlib.sha256(earth_source.read_bytes()).hexdigest()
provenance={'display_mask':'northern QD latitude >60 degrees; original footprint retained in HDF5','earth_texture_sha256':provenance_earth_hash,'source':cal['image_url'],'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'utc':cal['utc'],'emission_altitude_km':cal.get('emission_altitude_km',110),'calibration_quality':cal['quality'],'optmod':cal['optmod'],'lens':'existing Brown-Conrady star-track fit, not refitted','script':'scripts/prepare_smile.py','units':'processed RGB; not calibrated radiance','grid':'cell centres, latitude south to north; longitude -180 to 180','visible_pixels':int((alpha>0).sum())}
(out/'provenance.json').write_text(json.dumps(provenance,indent=2))
with h5py.File(out/'mapped.h5','w') as h:
    h.create_dataset('rgba',data=rgba,compression='gzip',shuffle=True)
    h['longitude_deg']=lon[0]*180/np.pi;h['latitude_deg']=lat[:,0]*180/np.pi
    h.attrs['provenance_json']=json.dumps(provenance)
    h.attrs['calibration_json']=json.dumps(cal)
assert (alpha>0).sum()>1000
print('SMILE map prepared:',provenance)
