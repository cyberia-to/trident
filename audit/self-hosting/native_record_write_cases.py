"""Installed fresh-JOB acceptance of persistent static field assignment."""

def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    import json
    vector=next(v for v in json.loads((repo.parent/'joy/cli/tests/artifact_vectors.json').read_text()) if v['name']=='identity_tree')
    input_file=root/'record-write-input.dag';input_file.write_bytes(bytes(vector['input']));value=decode(input_file)
    def source(decls,body):return f'program sample {decls} fn main(input:Noun)->Noun{{{body}}}'.encode()
    cases=[
        ('record-write-first-middle-last',source('struct S{x:Field,y:Field,z:Field}','let mut s=S{x:1,y:2,z:3} s.x=7 s.y=8 s.z=9 nox_noun_atom(s.x*100+s.y*10+s.z)'),789),
        ('record-write-snapshot',source('struct S{x:Noun,y:Noun}','let mut s=S{x:input,y:nox_noun_atom(9)} let old=s s.x=nox_noun_pair(s.x,s.y) nox_noun_pair(old.x,s.x)'),(value,(value,9))),
        ('record-write-nested',source('struct S{x:Field,y:Field} struct Box{left:S,right:S}','let mut b=Box{left:S{x:1,y:2},right:S{x:3,y:4}} let old=b b.left=b.right b.left.x=9 nox_noun_atom(old.left.x*1000+b.left.x*100+b.left.y*10+b.right.x)'),1943),
        ('record-write-call',source('struct S{x:Field,y:Field} fn update(a:S)->S{let mut s=a s.x=sub(s.x,s.y) s}','let original=S{x:9,y:2} let other=4 let changed=update(original) nox_noun_atom(original.x*100+changed.x*10+changed.y+other)'),976),
        ('record-write-rhs-call',source('struct S{x:Field,y:Field} fn replace(a:S)->Field{sub(a.x,a.y)}','let mut s=S{x:9,y:2} let old=s s.x=replace(s) nox_noun_atom(old.x*100+s.x*10+s.y)'),972),
        ('record-write-shadow',source('struct S{x:Field}','let mut s=S{x:7} if true{let mut s=S{x:3} s.x=9} nox_noun_atom(s.x)'),7),
        ('record-write-loop',source('struct S{x:Field}','let mut s=S{x:7} for i in 0..3{s.x=s.x+as_field(i)} nox_noun_atom(s.x)'),10),
        ('record-write-complete',source('struct S{d:Digest,t:(Field,Noun)}','let mut s=S{d:nox_noun_identity(nox_noun_atom(0)),t:(7,nox_noun_atom(0))} s.d=nox_noun_identity(input) s.t=(9,input) let(n,x)=s.t if s.d==nox_noun_identity(input){nox_noun_pair(nox_noun_atom(n),x)}else{nox_noun_atom(0)}'),(9,value)),
    ]
    for name,content,expected in cases:
        directory,job=package(name,content,{'arena_nodes':786432});program=directory/'program.dag';compiled=execute(job,program)
        assert compiled['execution']['compiler_job']['status']=='success'
        output=directory/'output.dag';executed=run(['run-artifact',program,'--input',input_file,'-o',output])
        assert decode(output)==expected,(name,decode(output),expected)
        assert executed['execution']['program_particle']==compiled['published_particle']
        observations.append({'case':name,'source_hex':content.hex(),'expected':expected,'input_particle':input_file.read_bytes()[8:40].hex(),'output_particle':output.read_bytes()[8:40].hex(),'compiler_execution':compiled,'program_execution':executed,'program_bytes':len(program.read_bytes()),'complete_output_checked':True})
    negatives=[
        ('record-write-immutable',source('struct S{x:Field}','let s=S{x:7} s.x=9 input')),
        ('record-write-parameter',source('struct S{x:Field} fn f(s:S)->S{s.x=9 s}','input')),
        ('record-write-immutable-shadow',source('struct S{x:Field}','let mut s=S{x:7} if true{let s=s s.x=9} input')),
        ('record-write-wrong-type',source('struct S{x:Field}','let mut s=S{x:7} s.x=true input')),
        ('record-write-call-root',source('struct S{x:Field} fn make()->S{S{x:7}}','make().x=9 input')),
        ('record-write-constructor-root',source('struct S{x:Field}','S{x:7}.x=9 input')),
        ('record-write-wrong-nominal',source('struct S{x:Field} struct T{x:Field} struct Box{x:S}','let mut b=Box{x:S{x:7}} b.x=T{x:9} input')),
        ('record-write-tuple-target',source('struct S{x:Field}','let mut s=S{x:7} (s.x,s.x)=(8,9) input')),
    ]
    for name,content in negatives:
        directory,job=package(name,content,{'arena_nodes':786432});compiled=execute(job,directory/'result.dag',emit='result')
        result=compiled['execution']['compiler_job'];assert result['status']=='compile_error' and len(result['diagnostics'])==1
        assert result['diagnostics'][0]['code']==5,(name,result)
        protected=directory/'protected.dag';protected.write_bytes(prior_program);execute(job,protected,expected=1,force=True)
        assert protected.read_bytes()==prior_program
        observations.append({'case':name,'source_hex':content.hex(),'diagnostics':result['diagnostics'],'compiler_execution':compiled,'previous_program_preserved':True})
    content=source('struct S{x:Noun}','let mut s=S{x:input} s.x=nox_noun_head(nox_noun_atom(1)) input')
    directory,job=package('record-write-rhs-trap',content,{'arena_nodes':786432});program=directory/'program.dag';compiled=execute(job,program)
    assert compiled['execution']['compiler_job']['status']=='success'
    output=directory/'output.dag';output.write_bytes(zero.read_bytes());run(['run-artifact',program,'--input',input_file,'-o',output,'--force'],expected=1)
    assert commands[-1]['stderr']=='error: execution failed: AxisError\n'
    assert output.read_bytes()==zero.read_bytes()
    observations.append({'case':'record-write-rhs-trap','source_hex':content.hex(),'compiler_execution':compiled,'program_execution_error':commands[-1]['stderr'],'previous_output_preserved':True})

    # Unchanged full-source vectors and arena ceiling from record_write_bounds.rs.
    # Terminal tree reads let the original 61–64-bit paths finish generation;
    # wider paths still preserve the previous output on arena exhaustion.
    for bits in [61,62,63,64,65]:
        outer=min(bits,64)-32
        fields=','.join(f'f{i}:Field' for i in range(31));values=','.join(f'f{i}:{i}' for i in range(31))
        prefix_fields=','.join(f'p{i}:Field' for i in range(outer-1));prefix_values=','.join(f'p{i}:{i}' for i in range(outer-1))
        inner=f'Inner{{{values},last:31}}';outer_value=f'Outer{{{prefix_values},last:{inner}}}'
        extra,value,path=('struct Wrap{value:Outer}',f'Wrap{{value:{outer_value}}}','w.value.last.last') if bits==65 else ('',outer_value,'w.last.last')
        content=f'program sample struct Inner{{{fields},last:Field}} struct Outer{{{prefix_fields},last:Inner}} {extra} fn main()->Field{{let mut w={value} let old=w {path}=99 {path.replace("w.","old.",1)}*100+{path}}}'.encode()
        name=f'record-write-wide-{bits}-arena';directory,job=package(name,content,{'arena_nodes':786432})
        if bits <= 64:
            program=directory/'program.dag';compiled=execute(job,program)
            assert compiled['execution']['compiler_job']['status']=='success'
            output=directory/'output.dag';executed=run(['run-artifact',program,'--input',zero,'-o',output])
            assert decode(output)==3199,(name,decode(output))
            assert executed['execution']['program_particle']==compiled['published_particle']
            observations.append({'case':name,'source_hex':content.hex(),'expected':3199,
                'compiler_execution':compiled,'program_execution':executed,
                'program_bytes':len(program.read_bytes()),'complete_output_checked':True})
            continue
        protected=directory/'program.dag';protected.write_bytes(prior_program);failure=execute(job,protected,expected=1,force=True)
        assert 'Unavailable' in commands[-1]['stderr'],commands[-1]
        assert protected.read_bytes()==prior_program
        observations.append({'case':name,'source_hex':content.hex(),'result':'valid source exceeds 786432-node compiler arena; full generation SH4 open','compiler_execution':failure,'previous_program_preserved':True})
