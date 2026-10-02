"""Validate exact retained originals without executing their code."""
import argparse,hashlib,json,re,stat
from pathlib import Path
ROOT=Path(__file__).resolve().parent
SECRET=re.compile(rb'(?:gh[pousr]_[A-Za-z0-9]{25,}|github_pat_[A-Za-z0-9_]{30,}|sk-proj-[A-Za-z0-9_-]{25,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)')
PROCESS=re.compile(rb'(?m)^\s*\d+\s+(?:\d+\s+){1,4}(?:[A-Z][a-z]{2})\s+(?:[A-Z][a-z]{2})\s+\d{1,2}\s+\d\d:\d\d:\d\d\s+\d{4}\s*$')
def require(value,message):
    if not value:raise ValueError(message)
def identity(path):
    require(stat.S_ISREG(path.lstat().st_mode),'ordinary retained file')
    data=path.read_bytes()
    return dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
def inspect_json(value):
    if isinstance(value,dict):
        require(not (isinstance(value.get('raw_stdout'),str) and value['raw_stdout']),'private raw process payload')
        for child in value.values():inspect_json(child)
    elif isinstance(value,list):
        for child in value:inspect_json(child)
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--originals',action='store_true');args=p.parse_args()
    rows=json.loads((ROOT/'retained-files.json').read_text());seen=set();objects=set()
    for row in rows:
        require(set(row)=={'original','bytes','sha256','stored'} and row['original'] not in seen,'exact distinct provenance entry')
        seen.add(row['original'])
        require(row['stored']=='objects/'+row['sha256'] and re.fullmatch('[0-9a-f]{64}',row['sha256']),'exact content-addressed object')
        obj=ROOT/row['stored'];expected={k:row[k] for k in ('bytes','sha256')}
        require(identity(obj)==expected,'retained object exact original identity');objects.add(row['stored'])
        if args.originals:require(identity(Path(row['original']))==expected,'original source remains identical')
        data=obj.read_bytes();require(not SECRET.search(data),'credential-shaped payload')
        require(len(PROCESS.findall(data))<3,'private full-host process table')
        if row['original'].endswith('.json'):inspect_json(json.loads(data))
    require({str(p.relative_to(ROOT)) for p in (ROOT/'objects').iterdir()}==objects,'no missing or unaccounted object')
    print(json.dumps(dict(status='passed-retained-byte-validation',original_files=len(rows),original_bytes=sum(r['bytes'] for r in rows),
          unique_objects=len(objects),stored_object_bytes=sum((ROOT/p).stat().st_size for p in objects),
          originals_checked=args.originals,map=identity(ROOT/'retained-files.json'),
          inspection='Credential-pattern, process-table and parsed nonempty raw_stdout checks passed. No original code was executed.')))
if __name__=='__main__':main()
