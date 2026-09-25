"""Fresh JOB1 acceptance for fixed Field arrays and checked native reads."""

def check(root, repo, run, package, execute, decode, record, observations, commands, zero, prior_program):
    def source(body, declarations=''):
        return f'program sample {declarations} fn main(input:Noun)->Noun{{{body}}}'.encode()
    cases=[
        ('array-empty',source('let a:[Field;0]=[] if a==[]{nox_noun_atom(7)}else{input}'),7),
        ('array-singleton',source('let a:[Field;1]=[7,] nox_noun_atom(a[0])'),7),
        ('array-field-u32',source('let a=[2,4,7] nox_noun_atom(a[0]*100+a[as_u32(1)]*10+a[2])'),247),
        ('array-typed-call',source('nox_noun_atom(id([7,9])[1])','fn id(a:[Field;2])->[Field;2]{a}'),9),
        ('array-tuple',source('let(a,b):([Field;2],Field)=([7,9],1) nox_noun_atom(a[b])'),9),
        ('array-snapshot',source('let mut a=[7,9] let old=a a=[3,4] nox_noun_atom(old[1]*10+a[0])'),93),
        ('array-record-write',source('let mut s=S{a:[7,9],b:2} let old=s s.a=[3,4] nox_noun_atom(old.a[0]*100+s.a[1]*10+s.b)','struct S{a:[Field;2],b:Field}'),742),
        ('array-nested-delimiters',source('nox_noun_atom(g([1,f(6)],([1,0])[0]))','fn f(x:Field)->Field{x+1} fn g(a:[Field;2],b:Field)->Field{a[b]}'),7),
        ('array-loop',source('let a=[1,2,3] let mut n=0 for i in 0..3{n=n+a[i]} nox_noun_atom(n)'),6),
        ('array-computed-normalized',source('nox_noun_atom([7][18446744069414584321+0])'),7),
        ('array-if-constructor',source('if [S{x:0}.x][0]{nox_noun_atom(7)}else{input}','struct S{x:Field}'),7),
    ]
    for n in [24,33,65]:
        values=','.join(map(str,range(n)))
        cases.append((f'array-wide-{n}',source(f'let a:[Field;{n}]=[{values}] nox_noun_atom(a[as_u32({n-1})])'),n-1))
    for name,content,expected in cases:
        directory,job=package(name,content,{'arena_nodes':786432});program=directory/'program.dag';compiled=execute(job,program)
        assert compiled['execution']['compiler_job']['status']=='success'
        output=directory/'output.dag';executed=run(['run-artifact',program,'--input',zero,'-o',output])
        assert decode(output)==expected,(name,decode(output),expected)
        assert executed['execution']['program_particle']==compiled['published_particle']
        observations.append({'case':name,'source_hex':content.hex(),'expected':expected,'compiler_execution':compiled,'program_execution':executed,'program_bytes':len(program.read_bytes()),'complete_output_checked':True})
    negatives=[
        ('array-length-type',source('let a:[Field;2]=[7] input'),5),
        ('array-literal-oob',source('nox_noun_atom([7][1])'),5),
        ('array-raw-literal-oob',source('nox_noun_atom([7][((18446744069414584321))])'),5),
        ('array-index-type',source('nox_noun_atom([7][true])'),5),
        ('array-wrong-delimiter',source('let a=[1,2) input'),2),
        ('array-empty-index',source('nox_noun_atom([7][])'),2),
        ('array-not-tuple',source('let(a,b)=[7,9] input'),5),
        ('array-element-write',source('let mut a=[7] a[0]=9 input'),2),
        ('array-extent-before-normalization',source('input','fn unused(a:[Field;18446744069414584321]){}'),7),
    ]
    for name,content,code in negatives:
        directory,job=package(name,content,{'arena_nodes':786432});compiled=execute(job,directory/'result.dag',emit='result')
        result=compiled['execution']['compiler_job'];assert result['status']=='compile_error' and len(result['diagnostics'])==1
        assert result['diagnostics'][0]['code']==code,(name,result)
        protected=directory/'protected.dag';protected.write_bytes(prior_program);execute(job,protected,expected=1,force=True)
        assert protected.read_bytes()==prior_program
        observations.append({'case':name,'source_hex':content.hex(),'diagnostics':result['diagnostics'],'compiler_execution':compiled,'previous_program_preserved':True})
    for name,content,error in [
        ('array-dynamic-oob',source('nox_noun_atom([7][nox_noun_as_field(input)+1])'),'InvZero'),
        ('array-empty-dynamic-oob',source('let a:[Field;0]=[] nox_noun_atom(a[nox_noun_as_field(input)])'),'InvZero'),
        ('array-base-before-index',source('nox_noun_atom([nox_noun_as_field(nox_noun_head(nox_noun_atom(1)))][as_field(as_u32(4294967296))])'),'AxisError'),
    ]:
        directory,job=package(name,content,{'arena_nodes':786432});program=directory/'program.dag';compiled=execute(job,program)
        assert compiled['execution']['compiler_job']['status']=='success'
        output=directory/'output.dag';output.write_bytes(zero.read_bytes());run(['run-artifact',program,'--input',zero,'-o',output,'--force'],expected=1)
        assert commands[-1]['stderr']==f'error: execution failed: {error}\n'
        assert output.read_bytes()==zero.read_bytes()
        observations.append({'case':name,'source_hex':content.hex(),'compiler_execution':compiled,'program_execution_error':commands[-1]['stderr'],'previous_output_preserved':True})
