//! B3 prototype: same static cache and trust boundaries as B2, plus a reverse
//! source/claim index. Complete request snapshots are diffed, with that cost exposed.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use veri_adam_cache::{Cache, CacheWork, Config, HeadCheck};
use veri_adam_protocol::*;
use veri_adam_verification::Report as VerificationReport;

#[derive(Debug, Default, Serialize)]
pub struct DependencyWork {
    pub snapshot_entries_examined: u64,
    pub snapshot_changes: u64,
    pub dependency_links_visited: u64,
    pub mismatch_changes: u64,
    pub metadata_lookups: u64,
    pub metadata_hits: u64,
    pub metadata_entries_inserted: u64,
    pub dependency_links_inserted: u64,
    pub cleanup_entries_examined: u64,
    pub metadata_entries_removed: u64,
    pub dependency_links_removed: u64,
    pub reset_claims_cleared: u64,
    pub reset_links_cleared: u64,
    pub reset_heads_cleared: u64,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub verification: VerificationReport,
    pub cache_work: CacheWork,
    pub dependency_work: DependencyWork,
    pub signature_entries: usize,
    pub decision_entries: usize,
    pub indexed_claims: usize,
    pub dependency_links: usize,
    pub tracked_sources: usize,
    pub observed_heads: usize,
}

#[derive(Default)]
struct Claim {
    expected: BTreeMap<String, String>,
    mismatched: BTreeSet<String>,
}

#[derive(Default)]
struct Index {
    heads: BTreeMap<String, String>,
    claims: BTreeMap<String, Claim>,
    by_source: BTreeMap<String, BTreeSet<String>>,
    links: usize,
}

impl Index {
    fn synchronize(&mut self, actual: &BTreeMap<String, String>, work: &mut DependencyWork) {
        // No trusted revision token or delta stream is assumed. Every supplied
        // full snapshot is examined, including absent keys and unrelated changes.
        let mut changed = Vec::new();
        for (source, old) in &self.heads {
            work.snapshot_entries_examined += 1;
            if actual.get(source) != Some(old) {
                changed.push(source.clone());
            }
        }
        for source in actual.keys() {
            work.snapshot_entries_examined += 1;
            if !self.heads.contains_key(source) {
                changed.push(source.clone());
            }
        }
        for source in changed {
            work.snapshot_changes += 1;
            let new = actual.get(&source);
            if let Some(bindings) = self.by_source.get(&source) {
                for binding in bindings {
                    work.dependency_links_visited += 1;
                    let claim = self
                        .claims
                        .get_mut(binding)
                        .expect("reverse index points to live metadata");
                    let was_mismatched = claim.mismatched.contains(&source);
                    let is_mismatched = claim.expected.get(&source) != new;
                    if is_mismatched {
                        claim.mismatched.insert(source.clone());
                    } else {
                        claim.mismatched.remove(&source);
                    }
                    work.mismatch_changes += u64::from(was_mismatched != is_mismatched);
                }
            }
            match new {
                Some(id) => {
                    self.heads.insert(source, id.clone());
                }
                None => {
                    self.heads.remove(&source);
                }
            }
        }
    }

    fn check(
        &mut self,
        binding: &str,
        expected: &[(String, String)],
        actual: &BTreeMap<String, String>,
        work: &mut DependencyWork,
    ) -> HeadCheck {
        work.metadata_lookups += 1;
        if let Some(claim) = self.claims.get(binding) {
            work.metadata_hits += 1;
            return HeadCheck {
                matched: claim.mismatched.is_empty(),
                comparisons: 0,
            };
        }
        // First eligible cache hit builds metadata from the already authenticated
        // certificate; nothing from a rejected or unverified bundle is admitted.
        let mut claim = Claim::default();
        for (source, id) in expected {
            if actual.get(source) != Some(id) {
                claim.mismatched.insert(source.clone());
            }
            claim.expected.insert(source.clone(), id.clone());
            self.by_source
                .entry(source.clone())
                .or_default()
                .insert(binding.into());
            self.links += 1;
            work.dependency_links_inserted += 1;
        }
        let matched = claim.mismatched.is_empty();
        self.claims.insert(binding.into(), claim);
        work.metadata_entries_inserted += 1;
        HeadCheck {
            matched,
            comparisons: expected.len() as u64,
        }
    }

    fn prune(&mut self, cache: &Cache, work: &mut DependencyWork) {
        // Eviction does not expose cache internals. Cleanup scans are charged.
        let mut stale = Vec::new();
        for binding in self.claims.keys() {
            work.cleanup_entries_examined += 1;
            if !cache.contains_decision(binding) {
                stale.push(binding.clone());
            }
        }
        for binding in stale {
            let claim = self.claims.remove(&binding).expect("stale metadata exists");
            work.metadata_entries_removed += 1;
            for source in claim.expected.keys() {
                let bindings = self
                    .by_source
                    .get_mut(source)
                    .expect("forward link has reverse link");
                assert!(bindings.remove(&binding));
                if bindings.is_empty() {
                    self.by_source.remove(source);
                }
                self.links -= 1;
                work.dependency_links_removed += 1;
            }
        }
    }
}

pub struct DependencyCache {
    cache: Cache,
    index: Index,
    reset_work: DependencyWork,
}
impl Default for DependencyCache {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
impl DependencyCache {
    pub fn new(config: Config) -> Self {
        Self {
            cache: Cache::new(config),
            index: Index::default(),
            reset_work: DependencyWork::default(),
        }
    }

    pub fn check(&mut self, b: &Bundle, p: &Policy, r: &Registry, q: &Request) -> Report {
        let mut work = std::mem::take(&mut self.reset_work);
        if q.kind == CheckKind::Current {
            self.index.synchronize(&q.heads, &mut work);
        }
        let index = &mut self.index;
        let cached =
            self.cache
                .check_with_head_checker(b, p, r, q, &mut |binding, expected, actual| {
                    index.check(binding, expected, actual, &mut work)
                });
        if cached.cache_work.decision_evictions > 0 {
            self.index.prune(&self.cache, &mut work);
        }
        Report {
            verification: cached.verification,
            cache_work: cached.cache_work,
            dependency_work: work,
            signature_entries: cached.signature_entries,
            decision_entries: cached.decision_entries,
            indexed_claims: self.index.claims.len(),
            dependency_links: self.index.links,
            tracked_sources: self.index.by_source.len(),
            observed_heads: self.index.heads.len(),
        }
    }

    pub fn clear(&mut self) {
        self.reset_work.reset_claims_cleared += self.index.claims.len() as u64;
        self.reset_work.reset_links_cleared += self.index.links as u64;
        self.reset_work.reset_heads_cleared += self.index.heads.len() as u64;
        self.cache.clear();
        self.index = Index::default();
    }
}
