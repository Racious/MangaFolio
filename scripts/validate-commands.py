"""Save complete current validation transcripts and exit codes. No database paths used."""
from pathlib import Path
import subprocess,json,datetime,os
root=Path(__file__).resolve().parents[1]
output=root/'docs/next-phase-results/final'
output.mkdir(parents=True,exist_ok=True)
commands=[('npm-test',['npm','test']),('npm-build',['npm','run','build']),('cargo-test',['cargo','test','--locked','--manifest-path','src-tauri/Cargo.toml']),('diff-check',['git','diff','--check'])]
results=[]
for name,command in commands:
 result=subprocess.run(command,cwd=root,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (output/f'{name}.log').write_text(result.stdout)
 record={'command':command,'exitCode':result.returncode,'output':f'{name}.log','executedAtUTC':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 results.append(record);print(json.dumps(record),flush=True)
(output/'exit-codes.json').write_text(json.dumps(results,indent=2)+'\n')
raise SystemExit(any(r['exitCode'] for r in results))
