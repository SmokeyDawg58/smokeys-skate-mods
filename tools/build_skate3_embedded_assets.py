"""Create the build-time player payload; runtime reads it directly from the EXE.

Keep original game assets outside this repository. Only the manifest, graphs,
collections, input and animation data needed by the captured player are packed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--localization', type=Path, required=True,
                        help='External extracted Skate 3 English language files')
    args = parser.parse_args()
    root = args.assets.resolve(strict=True)
    output = args.output.resolve()
    repository = Path(__file__).resolve().parents[1]
    if output.is_relative_to(repository):
        parser.error('The asset payload must stay outside the repository.')
    game = json.loads((root / 'private/game.json').read_text(encoding='utf-8'))
    paths = [
        'private/game.json', game['action_graph'], game['motion_graph'],
        'private/stock/skater-collections.json', 'private/stock/physics-skeletons.json',
        'private/stock/data/anim/OnBoard.abin', 'private/stock/data/anim/OffBoard.abin',
        'private/stock/data/anim/SkaterLeftHand.abin', 'private/stock/data/anim/SkaterRightHand.abin',
        'private/stock/data/config/input.cfg',
        'private/stock/data/script/camera/Default_cameragraph.stategraph',
        'private/stock/data/camera/1.shk', 'private/stock/data/camera/2.shk',
    ]
    paths += ['private/stock/data/joystick/' + name for name in
              ['skater.pat', 'skater90.pat', 'skaterN90.pat', 'skater_air.pat',
               'skater_fingerflip.pat', 'skaterls.pat', 'skaterstep.pat']]
    optional = 'private/custom/climbing.json'
    if (root / optional).is_file():
        paths.append(optional)
    paths = sorted(set(paths))
    localization_paths = []
    if args.localization:
        localization_paths = ['data/fe/languages/english/LANGUAGE_English_' + name + '_skate3ng.BIN'
                              for name in ['HISTOGRAM', 'Global']]
    paths += ['private/stock/' + name for name in localization_paths]
    records = []
    output.parent.mkdir(parents=True, exist_ok=True)
    # Publish only after every source path and byte payload was validated.
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as stream:
        temporary = Path(stream.name)
        try:
            stream.write(b'SK3PACK1' + struct.pack('<I', len(paths)))
            for relative in paths:
                if ('\\' in relative or ':' in relative or '\0' in relative or
                        any(p in ('', '.', '..') for p in relative.split('/'))):
                    raise ValueError('Unsafe asset path: ' + relative)
                language = relative.removeprefix('private/stock/')
                source = args.localization.resolve(strict=True) if language in localization_paths else root
                path = (source / (language if language in localization_paths else relative)).resolve(strict=True)
                path.relative_to(source)
                data = path.read_bytes()
                digest = hashlib.sha256(data).hexdigest()
                key = relative.encode('utf-8')
                stream.write(struct.pack('<IQ', len(key), len(data)))
                stream.write(digest.encode('ascii') + key + data)
                records.append({'path': relative, 'bytes': len(data), 'sha256': digest})
        except BaseException:
            stream.close()
            temporary.unlink()
            raise
    temporary.replace(output)
    manifest = {'version': 1, 'bytes': output.stat().st_size,
                'sha256': hashlib.sha256(output.read_bytes()).hexdigest(), 'files': records}
    output.with_suffix('.manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(json.dumps({'files': len(records), 'bytes': manifest['bytes'], 'sha256': manifest['sha256']}))


if __name__ == '__main__':
    main()
