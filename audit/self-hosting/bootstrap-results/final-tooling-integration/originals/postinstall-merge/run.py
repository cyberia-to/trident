import hashlib,json,os,pathlib,re,subprocess,time
ROOT=pathlib.Path(__file__).resolve().parent
REPO=ROOT.parent/'trident'
def identity(data):return dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
def output(argv,**kwargs):return subprocess.check_output(argv,**kwargs)
revision=output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip()
assert output(['git','status','--porcelain=v1'],cwd=REPO)==b''
prefix=ROOT/'prefix';assert not prefix.exists()
rustup=str(pathlib.Path.home()/'.cargo/bin/rustup')
paths={name:output([rustup,'which','--toolchain','1.89.0',name],text=True).strip() for name in ['cargo','rustc','rustdoc']}
env=os.environ.copy()
for key in list(env):
 if key.startswith(('RUST','CARGO_ENCODED')) or key in ['CC','CXX','AR','CFLAGS','CXXFLAGS','LDFLAGS']:
  env.pop(key)
env.update(PATH=f"{prefix}/bin:{pathlib.Path(paths['cargo']).parent}:{pathlib.Path.home()}/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",RUSTC=paths['rustc'],RUSTDOC=paths['rustdoc'],RUSTFLAGS='-D warnings',RUSTDOCFLAGS='-D warnings',CARGO_TARGET_DIR=str(ROOT.parent.parent/'census/joy/target'),CARGO_BUILD_JOBS='2',CARGO_INCREMENTAL='0')
versions={name:output([paths[name],'-vV'],env=env,text=True) for name in ['cargo','rustc']}
assert 'rustc 1.89.0 ' in versions['rustc'] and 'host: aarch64-apple-darwin' in versions['rustc']
assert 'cargo 1.89.0 ' in versions['cargo']
argv=[paths['cargo'],'install','--path','.','--root',str(prefix),'--locked','--offline','--force']
receipt=dict(status='running',scope='Committed clean source; fresh install prefix; inherited validated warm target cache; no Rust test rerun.',revision=revision,command=argv,cwd=str(REPO),environment={k:env[k] for k in ['PATH','RUSTC','RUSTDOC','RUSTFLAGS','RUSTDOCFLAGS','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_INCREMENTAL']},tool_paths=paths,tool_identities={name:identity(pathlib.Path(path).read_bytes()) for name,path in paths.items()},versions=versions,started_ns=time.time_ns())
(ROOT/'started.json').write_text(json.dumps(receipt,indent=2)+'\n')
start=time.monotonic()
with (ROOT/'stdout').open('xb') as stdout,(ROOT/'stderr').open('xb') as stderr:
 result=subprocess.run(argv,cwd=REPO,env=env,stdout=stdout,stderr=stderr,timeout=1800)
receipt.update(exit_code=result.returncode,elapsed_seconds=time.monotonic()-start,ended_ns=time.time_ns())
receipt['files']={name:identity((ROOT/name).read_bytes()) for name in ['stdout','stderr']}
receipt['warning_lines']=[line for name in ['stdout','stderr'] for line in (ROOT/name).read_text().splitlines() if re.search(r'\bwarning(?:\[|:)',line,re.I)]
receipt['clean_after']=output(['git','status','--porcelain=v1'],cwd=REPO)==b''
receipt['revision_after']=output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip()
receipt['binaries']={p.name:identity(p.read_bytes()) for p in sorted((prefix/'bin').glob('*')) if p.is_file()}
receipt['status']='passed' if result.returncode==0 and not receipt['warning_lines'] and receipt['clean_after'] and receipt['revision_after']==revision and set(receipt['binaries'])=={'trident','trident-lsp'} else 'failed'
(ROOT/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(dict(status=receipt['status'],revision=revision,seconds=receipt['elapsed_seconds'],binaries=receipt['binaries'])))
raise SystemExit(0 if receipt['status']=='passed' else 1)
