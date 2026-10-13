"""Archive inspected figure/export and browser checks as HDF5."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import h5py
import numpy as np
from PIL import Image

root = Path(__file__).resolve().parents[1]
browser = json.loads((root / 'validation/web.json').read_text())
assert browser['passed'] and not browser['errors']
hashes = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
          for folder in ('src', 'viewer/src', 'examples', 'scripts')
          for p in (root / folder).rglob('*') if p.is_file() and '__pycache__' not in str(p)}
with h5py.File(root / 'gallery/validation.h5', 'w') as f:
    f.attrs['passed'] = 1
    f.attrs['source_hashes_json'] = json.dumps(hashes, sort_keys=True)
    f.attrs['browser_checks_json'] = json.dumps(browser)
    f.attrs['visual_inspection'] = 'Exact PDF raster, native PNG, desktop/mobile browser, volume frames'
    f.attrs['examples'] = 'Synthetic fields and analytic vortex rings'
    for name, width in [('fields', 178.), ('single_column', 85.)]:
        g = f.create_group(name)
        info = subprocess.check_output(['pdfinfo', str(root / 'gallery' / f'{name}.pdf')], text=True)
        match = re.search(r'Page size:\s+([\d.]+) x ([\d.]+) pts', info)
        dims = np.array([float(v) for v in match.groups()])
        assert abs(dims[0] * 25.4 / 72 - width) < .01
        g['pdf_dimensions_pt'] = dims
        with Image.open(root / 'gallery' / f'{name}.png') as image:
            g['raster_dimensions_px'] = image.size
            g['raster_dpi'] = image.info['dpi']
            assert abs(image.info['dpi'][0] - 300) < .02
        for ext in ('pdf', 'png'):
            path = root / 'gallery' / f'{name}.{ext}'
            g.attrs[f'{ext}_sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    with Image.open(root / 'gallery/volume.gif') as image:
        f['volume_gif_dimensions_px'] = image.size
        f.attrs['volume_gif_frames'] = image.n_frames
        assert image.n_frames == 24
print('HDF5 validation: physical PDF dimensions, 300 dpi raster, browser checks and source hashes passed')
