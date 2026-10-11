#!/usr/bin/env python3
"""Record real stage timings, fresh-process RSS, and sizes (no comparative claims)."""
from pathlib import Path
import gzip, hashlib, json, os, platform, subprocess
from datetime import datetime, timezone
root = Path(__file__).resolve().parent.parent
os.chdir(root)
out = root / 'benchmarks/results'
out.mkdir(exist_ok=True)
subprocess.run(['cargo', 'build', '--release', '--locked', '-p', 'diagram-cli'], check=True)
timing = subprocess.run(['cargo', 'run', '--release', '--locked', '-p', 'diagram-render', '--example', 'benchmark'], capture_output=True, text=True, check=True)
(out / 'stages.csv').write_text(timing.stdout)
mem = []
for n in [8, 64, 256]:
    source = 'flowchart LR\n'
    for i in range(n):
        source += f'N{i}[Stage {i}]\n'
        if i: source += f'N{i-1}-->N{i}\n'
        if i > 3 and i % 4 == 0: source += f'N{i-3}-->N{i}\n'
    input_file = out / f'{n}.mmd'
    input_file.write_text(source)
    subprocess.run([str(root / 'target/release/diagram-rs'), str(input_file), '-o', str(out / f'{n}.svg')], check=True)
    rss = subprocess.check_output([str(root / 'target/release/examples/benchmark'), '--memory', str(n)], text=True)
    mem.append({'nodes': n, 'peak_rss_kib': int(rss), 'source_sha256': hashlib.sha256(source.encode()).hexdigest()})
sizes = {}
for name, path in [('native', root / 'target/release/diagram-rs'), ('wasm', root / 'crates/diagram-wasm/pkg/diagram_rs_bg.wasm')]:
    data = path.read_bytes()
    sizes[name] = {'bytes': len(data), 'gzip_bytes': len(gzip.compress(data, mtime=0))}
package_bytes = sum(p.stat().st_size for p in (root / 'crates/diagram-wasm/pkg').iterdir() if p.is_file())
model = next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), 'unknown')
report = {'date_utc': datetime.now(timezone.utc).isoformat(), 'platform': platform.platform(), 'cpu': model, 'available_cpus': len(os.sched_getaffinity(0)), 'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(), 'memory': mem, 'sizes': sizes, 'wasm_package_bytes': package_bytes, 'cargo_lock_sha256': hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest()}
(out / 'environment.json').write_text(json.dumps(report, indent=2) + '\n')
print(timing.stdout, end='')
print(json.dumps(report, indent=2))
