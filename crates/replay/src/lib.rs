//! Matched virtual-time replay: B1 and B2 receive the SAME trusted snapshots.
use serde::{Deserialize, Serialize};
use veri_adam_cache::{Cache, CacheWork, CachedReport, Config};
use veri_adam_protocol::*;
use veri_adam_verification::{Report, Work};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub version: u16,
    pub label: String,
    pub data_kind: String,
    pub cache_config: Config,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub label: String,
    pub reset_cache: bool,
    pub policy: Policy,
    pub registry: Registry,
    pub request: Request,
    pub bundle: Bundle,
    pub expected: Outcome,
}

#[derive(Debug, Serialize)]
pub struct StepReport {
    pub label: String,
    pub cache_reset: bool,
    pub expected: Outcome,
    pub reference: Outcome,
    pub b1: Report,
    pub b2: CachedReport,
    pub matched: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub checks: u64,
    pub mismatches: u64,
    pub cache_resets: u64,
    pub b1_work: Work,
    pub b2_work: Work,
    pub b2_cache_work: CacheWork,
    pub peak_signature_entries: usize,
    pub peak_decision_entries: usize,
}

#[derive(Debug, Serialize)]
pub struct ReplayReport {
    pub version: u16,
    pub workload_label: String,
    pub data_kind: String,
    pub cache_config: Config,
    pub all_matched: bool,
    pub summary: Summary,
    pub steps: Vec<StepReport>,
}

pub fn run(workload: &Workload) -> Result<ReplayReport, String> {
    if workload.version != VERSION || workload.steps.is_empty() || workload.steps.len() > 4096 {
        return Err("replay requires version 1 and 1..4096 steps".into());
    }
    if workload.data_kind != "synthetic" && workload.data_kind != "physical" {
        return Err("data_kind must explicitly be synthetic or physical".into());
    }
    let mut cache = Cache::new(workload.cache_config);
    let mut summary = Summary::default();
    let mut reports = Vec::new();
    for s in &workload.steps {
        if s.reset_cache {
            cache.clear();
            summary.cache_resets += 1;
        }
        let b1 = veri_adam_verification::check(&s.bundle, &s.policy, &s.registry, &s.request);
        let b2 = cache.check(&s.bundle, &s.policy, &s.registry, &s.request);
        let reference = veri_adam_reference::check(&s.bundle, &s.policy, &s.registry, &s.request);
        let matched = [b1.outcome, b2.verification.outcome, reference]
            .iter()
            .all(|o| *o == s.expected);
        summary.checks += 1;
        summary.mismatches += u64::from(!matched);
        add_work(&mut summary.b1_work, &b1.work);
        add_work(&mut summary.b2_work, &b2.verification.work);
        add_cache_work(&mut summary.b2_cache_work, &b2.cache_work);
        summary.peak_signature_entries = summary.peak_signature_entries.max(b2.signature_entries);
        summary.peak_decision_entries = summary.peak_decision_entries.max(b2.decision_entries);
        reports.push(StepReport {
            label: s.label.clone(),
            cache_reset: s.reset_cache,
            expected: s.expected,
            reference,
            b1,
            b2,
            matched,
        });
    }
    Ok(ReplayReport {
        version: VERSION,
        workload_label: workload.label.clone(),
        data_kind: workload.data_kind.clone(),
        cache_config: workload.cache_config,
        all_matched: summary.mismatches == 0,
        summary,
        steps: reports,
    })
}

fn add_work(total: &mut Work, n: &Work) {
    total.signature_checks += n.signature_checks;
    total.signature_cache_hits += n.signature_cache_hits;
    total.measurements_evaluated += n.measurements_evaluated;
    total.ballots_evaluated += n.ballots_evaluated;
}
fn add_cache_work(t: &mut CacheWork, n: &CacheWork) {
    t.decision_lookups += n.decision_lookups;
    t.decision_hits += n.decision_hits;
    t.decision_insertions += n.decision_insertions;
    t.decision_evictions += n.decision_evictions;
    t.signature_lookups += n.signature_lookups;
    t.signature_insertions += n.signature_insertions;
    t.signature_evictions += n.signature_evictions;
    t.checkpoint_comparisons += n.checkpoint_comparisons;
    t.key_bytes_hashed += n.key_bytes_hashed;
    t.key_items_sorted += n.key_items_sorted;
    t.certificate_items_examined += n.certificate_items_examined;
}

pub mod demo;
