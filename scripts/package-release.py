#!/usr/bin/env python3
"""Deterministic archive metadata and real checks of the contained release files."""
from pathlib import Path
import gzip, hashlib, io, json, os, platform, subprocess, tarfile, tempfile, tomllib

root = Path(__file__).resolve().parent.parent
version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
out = root / 'dist'
out.mkdir(exist_ok=True)
host = next(line.split(': ', 1)[1] for line in subprocess.check_output(['rustc', '-vV'], text=True).splitlines() if line.startswith('host: '))
if not host.endswith('linux-gnu'):
    raise SystemExit('Local native packaging is currently validated only for Linux GNU hosts')
binary = root / 'target/release/diagram-rs'
notices = {'LICENSE': root / 'LICENSE', 'THIRD_PARTY_LICENSES.txt': root / 'THIRD_PARTY_LICENSES.txt',
           'FONT-LICENSE.txt': root / 'assets/fonts/LICENSE.txt', 'NOTO-FONT-LICENSE.txt': root / 'assets/fonts/NotoSansJP-OFL.txt'}

def archive(path, files):
    with path.open('wb') as raw, gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as gz, tarfile.open(fileobj=gz, mode='w') as tar:
        for name, source in sorted(files.items()):
            data = source.read_bytes()
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o755 if os.access(source, os.X_OK) else 0o644
            info.mtime = info.uid = info.gid = 0
            tar.addfile(info, io.BytesIO(data))

prefix = f'diagram-rs-{version}'
native = out / f'{prefix}-{host}.tar.gz'
archive(native, {f'{prefix}/{name}': path for name, path in {'diagram-rs': binary, 'README.md': root / 'README.md', **notices}.items()})
source_files = {}
for path in root.rglob('*'):
    rel = path.relative_to(root)
    if any(part in {'.git', 'node_modules', 'target', 'dist', 'pkg', 'pkg-latin', '__pycache__'} for part in rel.parts):
        continue
    if rel.parts[:2] == ('benchmarks', 'results') or not path.is_file():
        continue
    source_files[f'{prefix}/{rel.as_posix()}'] = path
source = out / f'{prefix}-source.tar.gz'
archive(source, source_files)
npm = out / f'diagram-rs-{version}.tgz'
with tarfile.open(npm) as tar:
    names = set(tar.getnames())
    required = {f'package/{n}' for n in [*notices, 'index.js', 'index.d.ts', 'diagram_rs_bg.wasm', 'package.json', 'README.md']}
    if required - names:
        raise SystemExit(f'npm archive is missing: {required - names}')
    manifest = json.load(tar.extractfile('package/package.json'))
    assert manifest['version'] == version
    assert manifest['license'] == 'MIT'
with tempfile.TemporaryDirectory(prefix='diagram-rs-release-') as temp:
    # Extract only our regular files after checking each path, never archive links.
    with tarfile.open(native) as tar:
        for member in tar:
            rel = Path(member.name)
            assert member.isfile() and not rel.is_absolute() and '..' not in rel.parts
            path = Path(temp) / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(tar.extractfile(member).read())
            path.chmod(member.mode)
    executable = Path(temp) / prefix / 'diagram-rs'
    assert subprocess.check_output([str(executable), '--version'], text=True).strip() == f'diagram-rs {version}'
    fixture_names = sorted(p.stem for p in (root / 'tests/fixtures').glob('*.mmd'))
    for name in fixture_names:
        fixture = root / f'tests/fixtures/{name}.mmd'
        subprocess.run([str(executable), 'check', str(fixture)], check=True, stdout=subprocess.DEVNULL)
        svg = subprocess.check_output([str(executable), str(fixture)])
        assert svg == (root / f'tests/snapshots/{name}.svg').read_bytes(), f'archive CLI parity: {name}'
files = [native, source, npm]
records = {p.name: {'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()} for p in files}
try:
    source_commit = subprocess.check_output(['git', 'rev-parse', '--verify', 'HEAD'], cwd=root,
                                            stderr=subprocess.DEVNULL, text=True).strip()
    source_dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
except (subprocess.CalledProcessError, FileNotFoundError):
    # Building from the source archive is supported without a Git checkout.
    source_commit = source_dirty = None
(out / 'SHA256SUMS').write_text(''.join(f"{v['sha256']}  {k}\n" for k, v in sorted(records.items())))
(out / 'manifest.json').write_text(json.dumps({'version': version, 'host': host, 'platform': platform.platform(),
    'source_commit': source_commit, 'source_dirty': source_dirty,
    'verification': {'packaged_cli_fixture_parity': len(fixture_names), 'npm_required_files': sorted(required)},
    'artifacts': records, 'publication': 'unsigned; no registry or GitHub release publication'}, indent=2) + '\n')
print(f'Local archives verified: {len(fixture_names)} packaged CLI parity cases and npm license/entrypoint inventory')
