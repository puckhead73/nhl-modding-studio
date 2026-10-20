"""Restrict KD compatibility qualification to differentially executed inputs."""
import hashlib
import json
from pathlib import Path
from deke_runtime_evidence import SOURCE_HASH,TARGET_HASH

FIELDS=('PTMKDImpls','NumComponents','NumNNeighbours','UseKNN','UseAABB','SortKNNResults')

def digest(fields):
    data={name:fields[name] for name in FIELDS}
    return hashlib.sha256(json.dumps(data,sort_keys=True,separators=(',',':')).encode()).hexdigest()

def qualified(fields):
    certificate=json.loads(Path(__file__).with_name('ptm_kd_qualified.json').read_text())
    if certificate['source_sha256']!=SOURCE_HASH or certificate['target_sha256']!=TARGET_HASH:
        return False
    return digest(fields) in {row['sha256'] for row in certificate['databases']}
