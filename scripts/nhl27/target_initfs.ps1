# Read the user's NHL24 initfs using the key/IV consumed by its 1.60 loader.
# Key bytes remain in memory and are never printed or saved. No game writes.
param(
    [string]$Python = 'C:\Users\puckh\AppData\Local\Programs\Python\Python312\python.exe',
    [string]$Output = 'local/nhl27-eboot/compatibility'
)
$ErrorActionPreference = 'Stop'
$targetExe = 'F:\Games\NHL 24\game-UPDATE\eboot.bin'
if ((Get-FileHash -LiteralPath $targetExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne 'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9') {
    throw 'Unrecognized target executable; refusing build-specific key location'
}
$outputPath = (Resolve-Path -LiteralPath $Output).Path
# Decode the DbObject envelope with the same bounded reader used for source initfs.
@'
import sys
from pathlib import Path
sys.path.insert(0,'scripts/nhl27')
from dbobject import parse
root=Path(sys.argv[1])
for label,path in [('base',r'F:\Games\NHL 24\game\Data\initfs_Gen4b'),('patch',r'F:\Games\NHL 24\game-UPDATE\Patch\initfs_Gen4b')]:
    value=parse(path,True)
    (root/('target-initfs-'+label+'.encrypted')).write_bytes(value['encrypted'])
'@ | & $Python - $outputPath
if ($LASTEXITCODE -ne 0) { throw 'Envelope extraction failed' }
$stream = [IO.File]::OpenRead($targetExe)
$keyBytes = [byte[]]::new(16)
try {
    # VA 0xaa64080 -> SELF offset 0xa7c0ba0, verified for the hash above.
    [void]$stream.Seek(0xa7c0ba0, [IO.SeekOrigin]::Begin)
    if ($stream.Read($keyBytes, 0, 16) -ne 16) { throw 'Short key read' }
} finally { $stream.Dispose() }
try {
    foreach ($layerName in @('base','patch')) {
        $aes = [Security.Cryptography.Aes]::Create()
        try {
            $aes.Mode = [Security.Cryptography.CipherMode]::CBC
            $aes.Padding = [Security.Cryptography.PaddingMode]::PKCS7
            $aes.Key = $keyBytes
            $aes.IV = $keyBytes
            $ciphertext = [IO.File]::ReadAllBytes((Join-Path $outputPath "target-initfs-$layerName.encrypted"))
            $transform = $aes.CreateDecryptor()
            try { $plain = $transform.TransformFinalBlock($ciphertext, 0, $ciphertext.Length) }
            finally { $transform.Dispose() }
            [IO.File]::WriteAllBytes((Join-Path $outputPath "target-initfs-$layerName.dbobject"), $plain)
        } finally { $aes.Dispose() }
    }
} finally { [Array]::Clear($keyBytes,0,$keyBytes.Length) }
@'
import sys,json,hashlib
from pathlib import Path
sys.path.insert(0,'scripts/nhl27')
from dbobject import parse
root=Path(sys.argv[1]);receipt=[]
for label in ['base','patch']:
    files=parse(root/('target-initfs-'+label+'.dbobject'),True)
    for item in files:
        f=item.get('$file',{})
        if f.get('name') not in ['SharedTypeDescriptors.ebx','SharedTypeDescriptors_patch.ebx']:continue
        data=f['payload'];dest=root/('target-'+label+'-'+f['name'])
        dest.write_bytes(data)
        receipt.append(dict(layer=label,name=f['name'],size=len(data),sha256=hashlib.sha256(data).hexdigest(),path=str(dest)))
print(json.dumps(receipt,indent=2))
(root/'target-initfs-receipt.json').write_text(json.dumps(receipt,indent=2))
'@ | & $Python - $outputPath
if ($LASTEXITCODE -ne 0) { throw 'Decrypted DbObject validation failed' }
