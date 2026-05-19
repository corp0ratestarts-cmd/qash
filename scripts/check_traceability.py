#!/usr/bin/env python3
from __future__ import annotations
import argparse, datetime as dt, json, os, pathlib, re, subprocess, sys

ID_PATTERNS={k:re.compile(rf'^{k.upper()}-\d{{3,}}$') for k in ['rm','sp','cd','pf','ts','ar']}


def current_sha()->str:
    return subprocess.check_output(['git','rev-parse','--short=12','HEAD'],text=True).strip()


def parse_matrix(text:str):
    items=[]; cur=None
    for raw in text.splitlines():
        line=raw.strip()
        if not line or line.startswith('#'): continue
        if line.startswith('- rm:'):
            if cur: items.append(cur)
            cur={'rm':line.split(':',1)[1].strip()}
            continue
        if cur is None: continue
        if ':' in line:
            k,v=[x.strip() for x in line.split(':',1)]
            if k in {'status','summary'}:
                cur[k]=v.strip('"')
            elif k in {'sp','cd','pf','ts','ar'}:
                inner=v.strip().lstrip('[').rstrip(']')
                cur[k]=[x.strip() for x in inner.split(',') if x.strip()]
    if cur: items.append(cur)
    return items


def valid_list(item,key):
    vals=item.get(key)
    return isinstance(vals,list) and vals and all(isinstance(v,str) and ID_PATTERNS[key].match(v) for v in vals)


def main()->int:
    ap=argparse.ArgumentParser(); ap.add_argument('--matrix',default='docs/traceability/matrix.yaml'); ap.add_argument('--artifact-dir',default='artifacts/traceability'); args=ap.parse_args()
    items=parse_matrix(pathlib.Path(args.matrix).read_text())
    failures=[]; blockers=[]
    for it in items:
        rm=it.get('rm','<missing-rm>'); st=it.get('status','')
        if not isinstance(rm,str) or not ID_PATTERNS['rm'].match(rm): failures.append(f'{rm}: invalid RM id'); continue
        if st=='active':
            for k in ('sp','cd','pf','ts','ar'):
                if not valid_list(it,k): failures.append(f'{rm}: missing or invalid {k.upper()} chain')
            if not valid_list(it,'pf') or not valid_list(it,'ts'): blockers.append(rm)
    sha=os.getenv('GITHUB_SHA') or current_sha(); rid=sha[:12]
    out=pathlib.Path(args.artifact_dir); out.mkdir(parents=True,exist_ok=True)
    rep={'generated_at':dt.datetime.now(dt.timezone.utc).isoformat(),'sha':sha,'matrix':args.matrix,'active_items':[i.get('rm') for i in items if i.get('status')=='active'],'failures':failures,'rc_blockers':blockers,'ok':not failures}
    (out/f'{rid}.json').write_text(json.dumps(rep,indent=2)+'\n')
    md=['# Traceability Coverage Report','',f'- SHA: `{sha}`',f'- Matrix: `{args.matrix}`',f"- Generated (UTC): `{rep['generated_at']}`",'', '## Active RM items']
    md += [f'- {x}' for x in rep['active_items']]
    md += ['', '## Chain validation', '- ✅ Passed' if not failures else '- ❌ Failed']
    md += [f'  - {f}' for f in failures]
    md += ['', '## RC-tag rule', '- ✅ No RC blockers' if not blockers else '- ❌ Block RC tags: open RM missing PF or TS coverage']
    md += [f'  - {b}' for b in blockers]
    (out/f'{rid}.md').write_text('\n'.join(md)+'\n')
    if failures:
      print('Traceability check failed:\n'+'\n'.join(f' - {f}' for f in failures)); return 1
    print(f"Traceability check passed. Artifacts: {out / (rid + '.md')} and .json")
    return 0

if __name__=='__main__': raise SystemExit(main())
