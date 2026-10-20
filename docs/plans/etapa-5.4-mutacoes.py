"""Mutações controladas: executar isoladamente com rtk proxy python3 este_arquivo."""
from pathlib import Path
import hashlib, json, subprocess, time
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/mutations-5.4'
OUT.mkdir(exist_ok=True)
cases=[]
def add(name,file,old,new,test,package='logos-kernel',target='certify'):
    cases.append((name,file,old,new,test,package,target))
reg='crates/logos-kernel/src/registry.rs'
cert='crates/logos-kernel/src/certify.rs'
trans='crates/logos-kernel/src/transitions.rs'
add('skip_active_registry',reg,'if self.state(ledger, format, identity)? != (true, false) {','if false {','unknown_unregistered_revoked_and_wrong_format_do_not_authorize')
add('skip_registration_authority',reg,'event.body.verify_authority(&self.authority)?;','let _ = &self.authority;','forged_administrative_signature_cannot_activate_a_locally_known_checker')
add('skip_cake_execution',cert,'if format.as_str() == CAKE {','if false {','adr0030_binary_substitution_same_name_is_refused_by_certify')
add('skip_obligation_binding',cert,'if to_canon(&regenerated)? != bytes || hash_raw::<Obligation>(&bytes) != obligation {','if false {','signed_pending_cannot_bind_another_proposals_obligation')
add('reuse_concluded_pending',trans,'pending = None\n','let _ = check;\n','state15_16_certify_persists_verified_citable_and_reopens_pending_privately')
add('use_all_citations',cert,'used = projected\n                                .premises()\n                                .iter()\n                                .map(|r| Hash::from_bytes(*r.event_hash()))\n                                .collect()','used = proposal.cites.clone(); used.sort_unstable(); used.dedup(); let _ = projected;','used_is_derived_from_the_proof_and_revoked_citations_cannot_be_recovered')
add('skip_domain_source_guard',cert,'if vocabulary\n        .sorts()\n        .iter()\n        .any(|s| matches!(s.domain, SortDomain::Enumerated { .. }))','if false','enumeration_sources_are_not_promoted_by_hash_or_cast')
add('consistency_is_not_refutation',cert,'.check_refutation_from_source(','.check_consistency_from_source(','state15_refutation_requires_actual_accepted_model_and_active_registry')
add('rejected_event_allows_accepted','crates/logos-ledger/src/event_body.rs','if !matches!(check.outcome, CheckOutcome::Rejected(_)) {','if false {','certificate_rejection_has_only_v2_tag13_and_cannot_hold_accepted_or_unknown','logos-ledger','events')
add('rejection_projects_verified','crates/logos-ledger/src/index_project.rs','EventBody::CertificateRejected { check } => {\n            obligation = Some(check.obligation.as_bytes().as_slice());\n            PENDING','EventBody::CertificateRejected { check } => {\n            obligation = Some(check.obligation.as_bytes().as_slice());\n            VERIFIED','rejected_and_unknown_never_verify_and_rejected_can_retry_after_reopen')
originals={f:(ROOT/f).read_text() for _,f,*_ in cases}
sha=lambda s:hashlib.sha256(s.encode()).hexdigest()
report={'base':'d8f8fcc','original_sha256':{f:sha(s) for f,s in originals.items()},'cases':[]}
try:
    for name,f,old,new,test,package,target in cases:
        original=originals[f]
        assert old in original,(name,'missing mutation anchor')
        start=time.monotonic()
        cmd=['rtk','proxy','cargo','test','-p',package,'--test',target,test,'--','--exact','--nocapture']
        control=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=180)
        (OUT/(name+"-control.log")).write_text(control.stdout)
        assert control.returncode==0 and "1 passed; 0 failed" in control.stdout,(name,"positive control failed")
        (ROOT/f).write_text(original.replace(old,new,1))
        try:
            run=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=180)
        finally:
            (ROOT/f).write_text(original)
        (OUT/(name+'.log')).write_text(run.stdout)
        killed=run.returncode!=0 and 'test result: FAILED' in run.stdout and 'panicked at' in run.stdout and 'assertion' in run.stdout and 'Operation not permitted' not in run.stdout
        item=dict(name=name,file=f,test=test,command=cmd,exit=run.returncode,seconds=round(time.monotonic()-start,2),compiled_and_killed=killed,positive_control_exit=control.returncode,positive_control_sha256=sha(control.stdout),log_sha256=sha(run.stdout))
        report['cases'].append(item)
        (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(item),flush=True)
        if not killed: raise RuntimeError('mutant not demonstrably killed: '+name)
finally:
    for f,s in originals.items(): (ROOT/f).write_text(s)
    report['restored']=all((ROOT/f).read_text()==s for f,s in originals.items())
    (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print('MUTATIONS_OK',len(cases),'restored',report['restored'],flush=True)
