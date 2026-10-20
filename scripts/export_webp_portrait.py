"""Export one lossless library portrait to PNG, preserving exact RGBA pixels."""
import argparse
import io
from pathlib import Path
from PIL import Image

def export(source, destination):
    if destination.exists():
        raise FileExistsError(destination)
    with Image.open(source) as image:
        rgba = image.convert('RGBA')
        data = io.BytesIO()
        metadata = {k:image.info[k] for k in ['icc_profile','exif'] if k in image.info}
        rgba.save(data, 'PNG', compress_level=9, **metadata)
        with Image.open(io.BytesIO(data.getvalue())) as check:
            assert check.size == rgba.size and check.convert('RGBA').tobytes() == rgba.tobytes()
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as output:
            output.write(data.getvalue())

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path)
    args=parser.parse_args()
    export(args.source,args.destination)
