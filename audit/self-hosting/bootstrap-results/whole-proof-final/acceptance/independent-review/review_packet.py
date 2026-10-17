"""Independent bounded readback of the actual privacy-safe delivery packet."""
import importlib.util
import json
from pathlib import Path
import sys
import time
import review_actual as r

D=r.B/'whole-proof-final-delivery-preparation'
P=D/'staging/actual-1'
source=r.source_map(D,'sources.json','d2f216163f07add07ef7278d998656eccb2f1f60190322e085761191fa64f59f')
sys.path.insert(0,str(D))
import privacy


def main():
    receipt=r.load(P/'receipt.json')
    r.require(receipt['status']=='passed-local-packet','actual packet terminal pass')
    r.require(receipt['source']['sources']==source,'exact collector source closure')
    r.same(D/'sources.json',receipt['source']['manifest'])
    r.same(D/'independent-review.json',receipt['source']['review'])
    required=set(receipt['files'])|{'receipt.json'}
    actual={str(p.relative_to(P)) for p in P.rglob('*') if p.is_file()}
    r.require(actual==required,'exact packet inventory')
    total=0
    for name in sorted(required):
        data=r.read(P/name,receipt['files'].get(name));total+=len(data)
        privacy.public_bytes(data)
    r.require(r.read(P/'.gitattributes')==b'* -text\n','whole packet byte attributes')
    r.require(sum(x['bytes'] for x in receipt['files'].values())==receipt['staged_bytes_before_receipt'],'exact pre-receipt bytes')
    caps=receipt['caps']
    r.require(caps==dict(catalog_entries=20000,individual_read_bytes=67108864,output_bytes=67108864,
        public_stream_bytes=1048576,total_read_bytes=268435456,wall_seconds=120),'fixed packet limits')
    r.require(total<=caps['output_bytes'] and receipt['read_bytes']<=caps['total_read_bytes']
              and receipt['elapsed_seconds']<=caps['wall_seconds'],'actual packet counters fit fixed limits')
    rows=r.load(P/'retained-files.json');deps=r.load(P/'dependencies.json');summary=r.load(P/'summary.json')
    f=r.load(r.W/'final-review.json');w=r.load(r.W/'receipt.json')
    r.require(len(deps)==receipt['dependency_entries']==16703,'actual dependency entry total')
    r.require(len(rows)==receipt['streams']==94 and len({x['original'] for x in rows})==94,'exact distinct original stream count')
    expected=[]
    for g in f['generations']:
        n=g['generation']
        for action in ('selfbuild','fresh-verification'):
            expected.extend(str(r.B/f'whole-proof/attempts/c{n}-{action}-1'/stream) for stream in ('stdout','stderr'))
        for command in g['composite_cases']['actual_v5_commands']:
            expected.extend(str(Path(command['path']).parent/stream) for stream in ('stdout','stderr'))
    expected.extend(str(r.W/stream) for stream in ('stdout','stderr'))
    r.require({row['original'] for row in rows}==set(expected) and len(expected)==94,'only explicit approved actual command streams')
    objects=set()
    for row in rows:
        r.require(set(row)=={'original','bytes','sha256','stored'},'exact retained entry schema')
        r.require(row['stored']=='objects/'+row['sha256'] and row['bytes']<=caps['public_stream_bytes'],'bounded exact object name')
        original=r.read(Path(row['original']),row);stored=r.read(P/row['stored'],row)
        r.require(original==stored,'byte-exact original stream readback');privacy.public_bytes(original)
        objects.add(row['stored'])
    r.require(len(objects)==receipt['objects']==33 and {x for x in required if x.startswith('objects/')}==objects,'exact deduplicated object count')
    r.require(summary['original_F5']==r.reference(r.W/'final-review.json') and summary['original_watcher']==r.reference(r.W/'receipt.json'),'actual origin bindings')
    r.require(summary['native_commands']==42 and summary['capacity_permits']==23,'actual summary command/capacity counts')
    r.require(summary['profile']=='joy-nox-disclosed-compiler-v1' and summary['disclosure']=='complete public witness'
              and summary['physical_resource_claim']=='unattested','precise disclosed relation')
    r.require(summary['prior_statuses']=={'v2':'failed','v3':'failed','v4':'input-changed'},'summary preserves historical failures')
    r.require(len(summary['generations'])==2,'summary has two generations')
    for projected,g in zip(summary['generations'],f['generations']):
        c=g['composite_cases'];n=g['generation']
        for key in ('generation','proof','compiler','prove_seconds','verify_seconds','sampled_prove_rss_bytes','sampled_verify_rss_bytes','corpus_observations'):
            r.require(projected[key]==g[key],'exact summary metric '+key)
        r.require(projected['fresh_v5']==c['fresh_v5_rejections'] and projected['prior_v2']==c['reused_v2_rejections']
                  and projected['prior_v3']==c['reused_v3_rejections'],'exact projected case partition')
        r.require(projected['original_controls']==2 and projected['distinct_rejections']==23
                  and projected['selected_v4_cost']==(n==2),'exact composite total without relabeling')
    for name,identity in summary['source_bindings'].items():r.same(r.B/name,identity)
    r.require(all(isinstance(row.get('provenance'),list) and row['provenance'] for row in deps),'explicit dependency provenance')
    artifact_rows=[x for x in deps if x['classification']=='artifact-reference-no-body']
    r.require(all('stored' not in row for row in artifact_rows),'artifact identities remain references only')
    # Recheck all opened packet/source bytes at the end, without touching full proof or native bodies.
    for name,identity in list(r.seen.items()):
        path=Path(name)
        if path.is_relative_to(P) or path.parent==D:r.read(path,identity)
    return dict(schema='trident/actual-final-delivery-independent-review/v1',status='passed-actual-packet-review',
        packet_receipt=r.reference(P/'receipt.json'),F5=r.reference(r.W/'final-review.json'),watcher=r.reference(r.W/'receipt.json'),
        collector_sources=r.reference(D/'sources.json'),collector_review=r.reference(D/'independent-review.json'),
        files=len(required),original_streams=len(rows),unique_objects=len(objects),dependency_entries=len(deps),
        actual_packet_bytes=total,reported_collector_read_bytes=receipt['read_bytes'],reported_collector_elapsed_seconds=receipt['elapsed_seconds'],
        privacy='Every actual packet file and all 94 selected original streams passed the exact frozen strict public_bytes predicate. No raw process snapshots, proof/native bodies, authorization headers or signed URLs were copied by this review.',
        scope='Independent actual packet identity, source/gate, summary, counter, explicit original-stream allowlist and privacy readback. No collector, native proof, final checker, upload, version or publication action.')


if __name__=='__main__':
    report={'status':'failed','started_ns':time.time_ns(),'command':[sys.executable,*sys.argv]}
    try:report.update(main())
    except BaseException as error:
        report['error']=str(error)
        raise
    finally:
        report['ended_ns']=time.time_ns();report['review_read_bytes']=r.read_bytes
        report['observed_metadata_files']=len(r.seen)
        with (r.O/'packet-review-1.json').open('x') as out:json.dump(report,out,indent=2);out.write('\n')
        print(json.dumps({k:report[k] for k in ('status','review_read_bytes','observed_metadata_files')}))
