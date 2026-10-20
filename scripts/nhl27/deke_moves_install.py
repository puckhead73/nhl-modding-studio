"""Install/check a verified moves-only overlay without editing game originals."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

from asset_manifest import manifest
from bundle_inventory import bundles
from compatibility import frames, ps4_spans
from native_ant import Bank

IDENTITIES = {
    'nhl24.exe':'0da12e71bf4f0acea34e758f46b849ecd7f0b90a3e3570ddb1dc71dd5bfc01ce',
    'game/eboot.bin':'0118720c7e0338e20769063437be5656d1c16262c6b81a688adb659582544e17',
    'game-UPDATE/eboot.bin':'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9',
}


def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f,'sha256').hexdigest()


def safe(root, relative):
    result=(root/relative).resolve()
    if not result.is_relative_to(root.resolve()) or result==root.resolve():
        raise ValueError('Path escapes intended installation directory')
    return result


def listed(receipt):
    return {**{b['toc']:b['toc_sha256'] for b in receipt['banks']},
            **{c['path']:c['sha256'] for c in receipt['cas']},
            **({receipt['executable']['path']:receipt['executable']['sha256']} if 'executable' in receipt else {})}


def verify_overlay(overlay, receipt):
    expected=listed(receipt)
    actual={p.relative_to(overlay).as_posix() for p in overlay.rglob('*') if p.is_file()}
    if actual!=set(expected):
        raise ValueError('Overlay inventory differs from package')
    for rel,wanted in expected.items():
        if digest(safe(overlay,rel))!=wanted:
            raise ValueError(f'Overlay hash mismatch: {rel}')
    def span_bytes(span):
        if span['layer']!=1:
            raise ValueError('Expected patch resource')
        path=safe(overlay,f"Patch/Gen4b/superbundlelayout/nhl_installpackage_{span['catalog']:02d}/cas_{span['cas']:02d}.cas")
        if span['offset']+span['size']>path.stat().st_size:
            raise ValueError('Installed CAS span exceeds file')
        with path.open('rb') as f:
            f.seek(span['offset']);return f.read(span['size'])
    checks=[]
    for b in receipt['banks']:
        toc=safe(overlay,b['toc']);rows,_=bundles(toc);raw=toc.read_bytes()
        wrapper=0x22c if raw[:4]==bytes.fromhex('00d1ce01') else 0
        name='gen4b/levels/game/game' if b['bank']=='state' else 'gen4b/content/common/configs/bundles/launch_sba'
        row=next(r for r in rows if r['name']==name);start=wrapper+row['record_offset']
        spans=ps4_spans(raw[start:start+row['record_size']]);assets=manifest(span_bytes(spans[0]))
        candidates=[i for i,a in enumerate(assets) if a['kind']=='res' and a['name'].endswith('_antstate')]
        if len(candidates)!=1:raise ValueError('Ambiguous installed ANT bank')
        i=candidates[0];payload=span_bytes(spans[i+1]);decoded=frames(payload)[1]
        if hashlib.sha1(payload).hexdigest()!=assets[i]['sha1'] or hashlib.sha256(decoded).hexdigest()!=b['bank_sha256']:
            raise ValueError('Installed resource identity mismatch')
        bank=Bank(decoded)
        if struct.unpack('<4I',bytes.fromhex(assets[i]['resource_meta']))[:3]!=(
                bank.header,bank.sections['blob'][0]-bank.header,len(bank.blob)):
            raise ValueError('Installed ANT allocation metadata mismatch')
        checks.append(dict(bank=b['bank'],decoded_sha256=b['bank_sha256'],objects=len(bank.by_id),
                           manifest_hash=True,allocation_metadata=True))
    return checks


def identity_checks(target):
    for rel,wanted in IDENTITIES.items():
        if digest(safe(target,rel))!=wanted:raise ValueError(f'Installation identity changed: {rel}')
    game_path=target/'user/game_path.txt'
    if game_path.exists():
        configured=game_path.read_text().strip()
        if configured and Path(configured).resolve()!=(target/'game/eboot.bin').resolve():
            raise ValueError('Normal launcher selects a different game directory')


def run(mode,package,target):
    target=target.resolve();package=package.resolve()
    identity_checks(target)
    metadata=safe(target,'nhl27-moves-only');overlay=safe(target,'game-mods')
    if mode=='check':
        installed=json.loads((metadata/'installation.json').read_text())
        receipt=json.loads((metadata/'package-receipt.json').read_text())
        checks=verify_overlay(overlay,receipt)
        for rel,wanted in installed['protected_files'].items():
            if digest(safe(target,rel))!=wanted:raise ValueError(f'Protected file changed: {rel}')
        installed.update(installation_checks='passed',checked_utc=datetime.now(timezone.utc).isoformat(),
                         bank_checks=checks,active_overlay=True)
        (metadata/'installation.json').write_text(json.dumps(installed,indent=2))
        print(json.dumps(installed,indent=2));return
    # Refuse live replacement; no process is closed or interrupted by this tool.
    processes=subprocess.run(['tasklist','/FI','IMAGENAME eq nhl24.exe','/FO','CSV','/NH'],
        capture_output=True,text=True,check=True).stdout
    if '"nhl24.exe"' in processes.lower():raise ValueError('NHL24 is running; installation requires it closed')
    if overlay.exists() or metadata.exists():
        raise ValueError('Existing overlay/receipt must be reconciled before installation')
    receipt=json.loads((package/'package-receipt.json').read_text())
    if receipt['status']!='moves_only_package_verified_gameplay_untested':
        raise ValueError('Not a qualified moves-only package')
    integration=package.parent/'integration-receipt.json'
    if digest(integration)!=receipt['integration_receipt_sha256']:
        raise ValueError('Integration receipt changed')
    verify_overlay(package/'game-mods',receipt)
    if digest(package/'runtime-receipt.json')!=receipt['runtime_receipt_sha256']:
        raise ValueError('Runtime receipt differs from verified package')
    protected=dict(IDENTITIES)
    for rel in ['user/config.json','user/patches/nhl24.xml']:
        protected[rel]=digest(safe(target,rel))
    for rel in listed(receipt):
        protected['game-UPDATE/'+rel]=digest(safe(target,'game-UPDATE/'+rel))
    stage=safe(target,'game-mods.nhl27-moves-staging')
    if stage.exists():raise ValueError('Previous staging directory requires inspection')
    stage.mkdir()
    for rel in listed(receipt):
        dest=safe(stage,rel);dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(safe(package/'game-mods',rel),dest)
    checks=verify_overlay(stage,receipt)
    metadata.mkdir()
    shutil.copyfile(package/'package-receipt.json',metadata/'package-receipt.json')
    shutil.copyfile(integration,metadata/'integration-receipt.json')
    shutil.copyfile(package/'runtime-receipt.json',metadata/'runtime-receipt.json')
    # A same-parent rename activates the completely verified overlay atomically.
    stage.rename(overlay)
    result=dict(scope='NHL27 moves within NHL24 gameplay',active_overlay=True,
        installed_utc=datetime.now(timezone.utc).isoformat(),installation_checks='passed',
        game_root=str(target),overlay=str(overlay),bank_checks=checks,protected_files=protected,
        original_files_modified=False,executable_patches=receipt['executable_patches'],gameplay_tested=False,
        activation='Existing runtime mounts game-mods before game-UPDATE and game',
        rollback='Run rollback.ps1 to move this overlay into the receipt directory; no originals were overwritten')
    (metadata/'installation.json').write_text(json.dumps(result,indent=2))
    (metadata/'rollback.ps1').write_text(ROLLBACK)
    run('check',package,target)


ROLLBACK = r'''$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$overlay = Join-Path $root 'game-mods'
if (Get-Process nhl24 -ErrorAction SilentlyContinue) { throw 'Close NHL24 before rollback.' }
$receipt = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'package-receipt.json') -Raw | ConvertFrom-Json
$expected = @{}
foreach ($b in $receipt.banks) { $expected[$b.toc] = $b.toc_sha256 }
foreach ($c in $receipt.cas) { $expected[$c.path] = $c.sha256 }
if ($receipt.executable) { $expected[$receipt.executable.path] = $receipt.executable.sha256 }
$files = @(Get-ChildItem -LiteralPath $overlay -File -Recurse)
if ($files.Count -ne $expected.Count) { throw 'Overlay has other files; preserve them and reconcile before rollback.' }
foreach ($file in $files) {
    $rel = $file.FullName.Substring($overlay.Length + 1).Replace([char]92,[char]47)
    if (!$expected.ContainsKey($rel) -or (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected[$rel]) { throw "Overlay changed: $rel" }
}
$destination = Join-Path $PSScriptRoot ('disabled-overlay-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$resolvedSource = (Resolve-Path -LiteralPath $overlay).Path
$resolvedDestination = [IO.Path]::GetFullPath($destination)
$rootPrefix = $root + [IO.Path]::DirectorySeparatorChar
if (!$resolvedSource.StartsWith($rootPrefix,[StringComparison]::OrdinalIgnoreCase) -or !$resolvedDestination.StartsWith($rootPrefix,[StringComparison]::OrdinalIgnoreCase)) { throw 'Rollback path escaped installation.' }
Move-Item -LiteralPath $resolvedSource -Destination $resolvedDestination
$recordPath = Join-Path $PSScriptRoot 'installation.json'
$record = Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json
$record.active_overlay = $false
$record.installation_checks = 'rolled_back_overlay_preserved'
$record | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $recordPath
Write-Output 'Moves overlay disabled. Original NHL24 files remain intact.'
'''


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=['install','check'])
    parser.add_argument('package',type=Path)
    parser.add_argument('target',type=Path)
    args=parser.parse_args();run(args.mode,args.package,args.target)
