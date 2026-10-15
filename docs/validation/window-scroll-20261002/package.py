from pathlib import Path
import hashlib,json,subprocess,datetime,sys,importlib.util
repo=Path.cwd(); ev=repo/'docs/validation/window-scroll-20261002'
assert 'Built application at:' in (ev/'final-build.log').read_text(encoding='utf-8'), 'Build still running'
name='NHL Modding Studio rc-20261002-r75'
assert not list(Path('E:/').glob('*r75*')), 'r75 is occupied externally'
out=repo/'target/internal-candidates/nhlms-release-candidate-20261002-r75'
assert not out.exists(), 'r75 is occupied in the workspace'
out.mkdir(parents=True)
prior=Path('E:/nhlms-review-build-r73-2026-10-02/NHL Modding Studio rc-20261002-r73')
spec=importlib.util.spec_from_file_location('manifest',repo/'tools/review-build/manifest.py'); m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.inputs(str(ev/'dirty-inputs-after.tsv'))
assert (ev/'dirty-inputs-before.tsv').read_bytes()==(ev/'dirty-inputs-after.tsv').read_bytes(), 'Source drift'
def sha(p):return m.sha(str(p))
def tree(d,slug):
 m.tree(str(d),str(ev/(slug+'.tsv')))
 rows=(ev/(slug+'.tsv')).read_text(encoding='utf-8').splitlines()[1:]
 return hashlib.sha256(''.join(f'{r.split(chr(9))[0]}\t{r.split(chr(9))[-1]}\n' for r in rows).encode()).hexdigest()
rec=json.loads((repo/'docs/validation/evidence/overnight-development-2026-09-24/review-build-r73/build-record-input.json').read_text(encoding='utf-8'))
exe=repo/'target/release/nhl-studio.exe'
rec.update(command="npm run build (existing release profile and build cache)",candidate=name,built=(ev/'build-start.txt').read_text(encoding='utf-8-sig').strip()+' .. '+datetime.datetime.now().astimezone().isoformat(),head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),dirty_inputs=len((ev/'dirty-inputs-before.tsv').read_text(encoding='utf-8').splitlines())-1,dirty_manifest_sha256=sha(ev/'dirty-inputs-before.tsv'),dirty_inputs_after_build_identical=True,exe_sha256=sha(exe),exe_bytes=exe.stat().st_size,defs_tree_sha256=tree(repo/'defs','defs-tree'),frontend_dist_tree_sha256=tree(repo/'app/dist','frontend-tree'),anchors_tree_sha256=tree(repo/'anchors','anchors-tree'),helpers_tree_sha256=tree(prior/'helpers','helpers-tree'),baseline_inventory_sha256=sha(repo/'baselines/INVENTORY.json'),included_receipts='r74 plus bounded scrollable onboarding panel; see docs/validation/window-scroll-20261002. Full roadmap remains incomplete.',launched=False,published=False)
(ev/'build-record-input.json').write_text(json.dumps(rec,indent=2),encoding='utf-8')
subprocess.run([sys.executable,str(repo/'tools/release-archive/assemble.py'),'--exe',str(exe),'--helper-bundle',str(prior),'--out',str(out),'--name',name,'--build-record',str(ev/'build-record-input.json')],check=True)



