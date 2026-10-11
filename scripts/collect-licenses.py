#!/usr/bin/env python3
"""Collect upstream notices for the locked Rust dependency set, including dev deps."""
import json, subprocess
from pathlib import Path
root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=root))
notices = ['Third-party Rust dependency notices\n\nGenerated from Cargo.lock. Includes development-only crates conservatively.\nProject code is MIT; the separate bundled font notice is in assets/fonts/LICENSE.txt.\n']
missing = []
unique = {}
for package in sorted(metadata['packages'], key=lambda p:(p['name'],p['version'])):
    if package['id'] in metadata['workspace_members']: continue
    folder = Path(package['manifest_path']).parent
    files = [p for p in folder.iterdir() if p.is_file() and p.name.lower().startswith(('license','licence','copying','copyright'))]
    if package.get('license_file'):
        files.append(folder / package['license_file'])
    for sub in ['licenses','LICENSES']:
        if (folder/sub).is_dir(): files.extend(p for p in (folder/sub).rglob('*') if p.is_file())
    files = sorted(set(files))
    notices.append(f"\n{'='*72}\n{package['name']} {package['version']}\nSPDX: {package['license']}\nSource: {package.get('repository') or 'https://crates.io/crates/'+package['name']}\n")
    if not files: missing.append(package['name'])
    for file in files:
        text = file.read_text()
        if text not in unique: unique[text] = f'Notice {len(unique)+1}'
        notices.append(f'  {file.relative_to(folder)}: {unique[text]}\n')
if missing: raise SystemExit('Upstream notice files missing: '+', '.join(missing))
for text, label in unique.items(): notices.append(f'\n{chr(61)*72}\n{label}\n\n{text}\n')
(root/'THIRD_PARTY_LICENSES.txt').write_text(''.join(notices))
print('Wrote upstream notices for',len(metadata['packages'])-len(metadata['workspace_members']),'locked Rust dependencies')
