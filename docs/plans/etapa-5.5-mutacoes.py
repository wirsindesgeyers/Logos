from pathlib import Path
import hashlib,json,subprocess,time
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'target/mutations-5.5-final';OUT.mkdir(exist_ok=True)
cases=[]
def add(name,file,old,new,test,package='logos-kernel',target='certify'):
    cases.append((name,file,old,new,test,package,target))
w='crates/logos-kernel/src/witness.rs';c='crates/logos-kernel/src/certify.rs'
before='witness_before_certify_survives_restart_and_is_not_a_countermodel'
after='witness_after_certify_preserves_signed_bytes_and_failed_retry_cannot_downgrade'
add('accept_inconsistent_model',w,'        let tag = match outcome {','        let outcome = CheckOutcome::Accepted;\n        let tag = match outcome {','inconsistent_gamma_proves_arbitrary_goal_but_never_authorizes_action')
add('confuse_consistency_with_refutation',w,'.check_consistency_from_source(','.check_refutation_from_source(',before)
add('unknown_can_authorize',w,'            Consistency::Witness { .. }\n        ))','            Consistency::Unknown { .. }\n        ))',after)
add('ignore_late_annex',w,'&& *target == hash(&c.target)?','&& false',after)
add('erase_witness_on_failed_retry',w,'} else if !matches!(c.consistency, Consistency::Witness { .. }) {','} else {',after)
add('inherit_prior_instance_witness',w,'                current = Some(Instance {','                let inherited = current.as_ref().map(|c| c.consistency.clone()).unwrap_or(Consistency::Unknown { check: None });\n                current = Some(Instance {','witness_rejects_stale_epoch_revoked_checker_and_other_instance')
# This case needs a second replacement inside the instance initializer.
add('ignore_invalidated_target',w,'|| !eligible(hash(&c.target)?)?','|| false','witness_missing_context_and_invalidation_never_create_consistency_authority')
add('auditor_ignores_target','crates/logos-audit/src/verify.rs','&& claims.get(&target) != Some(&(obligation, p.epoch))','&& false','independent_auditor_rejects_wrong_annex_target_and_forged_accepted_model')
add('wrong_body_version','crates/logos-ledger/src/event_body_codec.rs','Value::Unsigned(3),\n            Value::Unsigned(14),','Value::Unsigned(2),\n            Value::Unsigned(14),','consistency_body_v3_rejects_wrong_version_arity_and_hash_sizes','logos-ledger','events')
add('forget_selected_instance',w,'selected = reference == Some(hash(event)?);','selected = false;',before)
originals={f:(ROOT/f).read_text() for _,f,*_ in cases}
sha=lambda s:hashlib.sha256(s.encode()).hexdigest()
report={'base':'12826f4','original_sha256':{f:sha(s) for f,s in originals.items()},'cases':[]}
if (OUT/'report.json').exists():
    previous=json.loads((OUT/'report.json').read_text())
    assert previous['original_sha256']==report['original_sha256'] and previous.get('restored')
    report=previous
completed={item['name'] for item in report['cases'] if item['compiled_and_killed']}
try:
    for name,f,old,new,test,package,target in cases:
        if name in completed: continue
        original=originals[f];assert old in original,(name,'missing anchor')
        cmd=['rtk','proxy','cargo','test','-p',package,'--test',target,test,'--','--exact','--nocapture']
        started=time.monotonic();control=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
        (OUT/(name+'-control.log')).write_text(control.stdout)
        assert control.returncode==0 and '1 passed; 0 failed' in control.stdout,(name,'control failed')
        mutated=original.replace(old,new,1)
        if name=='inherit_prior_instance_witness':
            mutated=mutated.replace('consistency: Consistency::Unknown { check: None },','consistency: inherited,',1).replace('.unwrap_or(Consistency::Unknown { check: None })','.map_or(Consistency::Unknown { check: None }, |x| x)')
        (ROOT/f).write_text(mutated)
        try:run=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
        finally:(ROOT/f).write_text(original)
        (OUT/(name+'.log')).write_text(run.stdout)
        killed=run.returncode!=0 and 'test result: FAILED' in run.stdout and 'panicked at' in run.stdout and 'could not compile' not in run.stdout and 'Operation not permitted' not in run.stdout
        item={'name':name,'file':f,'test':test,'exit':run.returncode,'positive_control_exit':control.returncode,'compiled_and_killed':killed,'seconds':round(time.monotonic()-started,2),'log_sha256':sha(run.stdout),'positive_control_sha256':sha(control.stdout)}
        report['cases'].append(item);(OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(item),flush=True)
        if not killed:raise RuntimeError('not demonstrably killed: '+name)
finally:
    for f,s in originals.items():(ROOT/f).write_text(s)
    report['restored']=all((ROOT/f).read_text()==s for f,s in originals.items());(OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print('MUTATIONS_OK',len(cases),'restored',report['restored'],flush=True)
