//! B2: bounded ordinary caches for positive signatures and static decision checks.
//! Time and required evidence heads are checked on every lookup, never cached as truth.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use veri_adam_protocol::*;
use veri_adam_verification::{Authentication, Report, Work, check_with_authenticator};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub signature_capacity: usize,
    pub decision_capacity: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            signature_capacity: 1024,
            decision_capacity: 256,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CacheWork {
    pub decision_lookups: u64,
    pub decision_hits: u64,
    pub decision_insertions: u64,
    pub decision_evictions: u64,
    pub signature_lookups: u64,
    pub signature_insertions: u64,
    pub signature_evictions: u64,
    pub checkpoint_comparisons: u64,
    pub key_bytes_hashed: u64,
    pub key_items_sorted: u64,
    pub certificate_items_examined: u64,
}

#[derive(Debug, Serialize)]
pub struct CachedReport {
    pub verification: Report,
    pub cache_work: CacheWork,
    pub signature_entries: usize,
    pub decision_entries: usize,
}

/// Result from a trusted checkpoint-checking backend. `comparisons` counts actual
/// source/head equality checks; other maintenance must be reported by that backend.
pub struct HeadCheck {
    pub matched: bool,
    pub comparisons: u64,
}

pub struct Cache {
    signatures: Fifo<()>,
    decisions: Fifo<Certificate>,
}
impl Default for Cache {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
impl Cache {
    pub fn new(config: Config) -> Self {
        Self {
            signatures: Fifo::new(config.signature_capacity),
            decisions: Fifo::new(config.decision_capacity),
        }
    }

    pub fn check(&mut self, b: &Bundle, p: &Policy, r: &Registry, q: &Request) -> CachedReport {
        self.check_with_head_checker(b, p, r, q, &mut |_, expected, actual| HeadCheck {
            matched: expected.iter().all(|(source, id)| actual.get(source) == Some(id)),
            // Evaluate all required heads for consistent accounting, even after a mismatch.
            comparisons: expected.len() as u64,
        })
    }

    /// Backend is trusted verifier code and must check the exact supplied snapshot.
    /// Static authentication, bindings, quorum and time checks remain in this crate.
    pub fn check_with_head_checker(
        &mut self, b: &Bundle, p: &Policy, r: &Registry, q: &Request,
        head_check: &mut impl FnMut(&str, &[(String, String)], &BTreeMap<String, String>) -> HeadCheck,
    ) -> CachedReport {
        let mut work = CacheWork::default();
        // Bind ALL contents, including signatures and complete registry contents, not merely version labels.
        // now/kind/heads are excluded because they are re-evaluated on every hit.
        // Transport array order is immaterial for otherwise valid unique records.
        // Normalize it so merely reordering a bundle or registry does not force rule recomputation.
        let mut measurements: Vec<_> = b.measurements.iter().collect();
        measurements.sort_by(|a, b| a.payload.signer_id.cmp(&b.payload.signer_id));
        let mut ballots: Vec<_> = b.ballots.iter().collect();
        ballots.sort_by(|a, b| a.payload.voter_id.cmp(&b.payload.voter_id));
        let mut members: Vec<_> = r.members.iter().collect();
        members.sort_by(|a, b| a.signer_id.cmp(&b.signer_id));
        work.key_items_sorted += (measurements.len() + ballots.len() + members.len()) as u64;
        let key = key(
            "veri-adam/b2-decision/v1",
            &(
                (&b.decision, measurements, ballots),
                p,
                (r.version, members),
                &q.event_id,
                &q.nonce,
            ),
            &mut work,
        );
        work.decision_lookups += 1;
        let verification = if let Some(certificate) = self.decisions.entries.get(&key) {
            work.decision_hits += 1;
            certificate.recheck(&key, q, &mut work, head_check)
        } else {
            let signatures = &mut self.signatures;
            let report =
                check_with_authenticator(b, p, r, q, &mut |bytes, signature, public_key| {
                    work.signature_lookups += 1;
                    let key = key_for_signature(bytes, signature, public_key, &mut work);
                    if signatures.entries.contains_key(&key) {
                        return Ok(Authentication::Reused);
                    }
                    verify(bytes, signature, public_key)?;
                    let (inserted, evicted) = signatures.insert(key, ());
                    work.signature_insertions += u64::from(inserted);
                    work.signature_evictions += u64::from(evicted);
                    Ok(Authentication::Checked)
                });
            // Rejected bundles do not establish reusable decision certificates.
            // Positive signature entries from a partially rejected bundle are still valid signature facts.
            if report.outcome != Outcome::Rejected {
                work.certificate_items_examined += (b.measurements.len() + b.ballots.len()) as u64;
                let (inserted, evicted) = self.decisions.insert(key, Certificate::new(b, &report));
                work.decision_insertions += u64::from(inserted);
                work.decision_evictions += u64::from(evicted);
            }
            // B1 performs one required-head comparison per fully evaluated measurement.
            work.checkpoint_comparisons += report.work.measurements_evaluated;
            report
        };
        CachedReport {
            verification,
            cache_work: work,
            signature_entries: self.signatures.entries.len(),
            decision_entries: self.decisions.entries.len(),
        }
    }

    /// Forget all cached work, as a new process would. No acceptance state survives restart.
    pub fn clear(&mut self) {
        self.signatures.clear();
        self.decisions.clear();
    }

    /// Allows an auxiliary index to discard metadata for evicted certificates.
    pub fn contains_decision(&self, binding: &str) -> bool {
        self.decisions.entries.contains_key(binding)
    }
}

#[derive(Clone)]
struct Certificate {
    decision_id: String,
    horizon: u64,
    latest_capture: u64,
    has_quorum: bool,
    heads: Vec<(String, String)>,
}
impl Certificate {
    fn new(b: &Bundle, report: &Report) -> Self {
        Self {
            decision_id: report
                .decision_id
                .clone()
                .expect("only successful static checks are cached"),
            horizon: report
                .support_expires_at_ms
                .expect("validated support horizon"),
            latest_capture: b
                .measurements
                .iter()
                .map(|m| m.payload.captured_at_ms)
                .max()
                .expect("nonempty validated source set"),
            has_quorum: b
                .ballots
                .iter()
                .filter(|v| v.payload.advisory == b.decision.payload.advisory)
                .count()
                >= 2,
            heads: b
                .measurements
                .iter()
                .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
                .collect(),
        }
    }

    fn recheck(
        &self, binding: &str, q: &Request, work: &mut CacheWork,
        head_check: &mut impl FnMut(&str, &[(String, String)], &BTreeMap<String, String>) -> HeadCheck,
    ) -> Report {
        let (outcome, reason) = if q.now_ms < self.latest_capture {
            (Outcome::Rejected, "future_measurement")
        } else if !self.has_quorum {
            (Outcome::Unconfirmed, "insufficient_matching_ballots")
        } else if q.kind == CheckKind::Historical {
            (
                Outcome::VerifiedHistorical,
                "historical_processing_verified",
            )
        } else if q.now_ms >= self.horizon {
            (Outcome::Expired, "support_expired")
        } else {
            let checked = head_check(binding, &self.heads, &q.heads);
            work.checkpoint_comparisons += checked.comparisons;
            if checked.matched {
                (Outcome::VerifiedCurrent, "current_support_verified")
            } else {
                (Outcome::Unconfirmed, "missing_or_replaced_trusted_head")
            }
        };
        Report {
            outcome,
            reason: reason.into(),
            decision_id: (outcome != Outcome::Rejected).then(|| self.decision_id.clone()),
            support_expires_at_ms: (outcome != Outcome::Rejected).then_some(self.horizon),
            work: Work::default(),
        }
    }
}

fn key<T: Serialize>(domain: &str, value: &T, work: &mut CacheWork) -> String {
    let bytes = serde_json::to_vec(&(domain, value))
        .expect("protocol values contain no fallible JSON values");
    work.key_bytes_hashed += bytes.len() as u64;
    digest(&bytes)
}
fn key_for_signature(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
    work: &mut CacheWork,
) -> String {
    key(
        "veri-adam/b2-signature/v1",
        &(bytes, signature, public_key),
        work,
    )
}

/// Deterministic bounded FIFO eviction. Zero capacity disables retention.
struct Fifo<T> {
    capacity: usize,
    entries: BTreeMap<String, T>,
    order: VecDeque<String>,
}
impl<T> Fifo<T> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }
    fn insert(&mut self, key: String, value: T) -> (bool, bool) {
        if self.capacity == 0 || self.entries.contains_key(&key) {
            return (false, false);
        }
        let evicted = self.entries.len() == self.capacity;
        if evicted {
            let oldest = self.order.pop_front().expect("FIFO occupancy");
            self.entries.remove(&oldest);
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, value);
        (true, evicted)
    }
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}
