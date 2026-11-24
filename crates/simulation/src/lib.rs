//! Deterministic virtual-time claim-delivery simulation. Toy environmental
//! trajectories and public fixture keys; not a calibrated physical room model.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use veri_adam_protocol::*;
use veri_adam_replay::{ReplayReport, Step, Workload};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    Stable,
    Dropout,
    Partition,
    Delay,
    Tamper,
    Replay,
    FalseReading,
    Mixed,
}
impl Scenario {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s { "stable"=>Ok(Self::Stable),"dropout"=>Ok(Self::Dropout),"partition"=>Ok(Self::Partition),
            "delay"=>Ok(Self::Delay),"tamper"=>Ok(Self::Tamper),"replay"=>Ok(Self::Replay),
            "false-reading"=>Ok(Self::FalseReading),"mixed"=>Ok(Self::Mixed),
            _=>Err("scenario must be stable, dropout, partition, delay, tamper, replay, false-reading or mixed".into()) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub seed: u64,
    pub scenario: Scenario,
    pub duration_ms: u64,
    pub sample_interval_ms: u64,
    pub check_interval_ms: u64,
    pub transport_delay_ms: u64,
    pub evidence_ttl_ms: u64,
    pub deadline_ms: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 42,
            scenario: Scenario::Mixed,
            duration_ms: 60_000,
            sample_interval_ms: 5_000,
            check_interval_ms: 1_000,
            transport_delay_ms: 250,
            evidence_ttl_ms: 10_000,
            deadline_ms: 2_000,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(20_000..=300_000).contains(&self.duration_ms)
            || self.sample_interval_ms < 1_000
            || self.sample_interval_ms > self.duration_ms / 4
            || self.check_interval_ms < 250
            || self.check_interval_ms > self.duration_ms
            || self.transport_delay_ms > 60_000
            || self.evidence_ttl_ms == 0
            || self.evidence_ttl_ms > 60_000
            || self.deadline_ms == 0
            || self.deadline_ms > 60_000
        {
            return Err("invalid simulation bounds: duration 20..300 s, sample cadence >=1 s and <=duration/4, checks >=250 ms, TTL/deadline 1..60000 ms, delay <=60000 ms".into());
        }
        if self.duration_ms / self.check_interval_ms
            + self.duration_ms / self.sample_interval_ms * 3
            + 8
            > 4096
        {
            return Err("simulation would exceed replay step bound".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub captured_at_ms: u64,
    pub sequence: u64,
    pub model_co2_ppm: u32,
    pub model_light_lux: u32,
    pub measured_a_ppm: Option<u32>,
    pub measured_b_ppm: u32,
    pub authentic_false_reading: bool,
}
#[derive(Debug, Serialize)]
pub struct Frame {
    pub time_ms: u64,
    pub model_co2_ppm: u32,
    pub model_light_lux: u32,
    pub pi_a_online: bool,
    pub network_connected: bool,
}
#[derive(Debug, Serialize)]
pub struct Trace {
    pub time_ms: u64,
    pub kind: String,
    pub detail: String,
    pub sequence: Option<u64>,
    pub check_index: Option<usize>,
}
#[derive(Debug, Default, Serialize)]
pub struct SimulationSummary {
    pub attempted_rounds: u64,
    pub created_rounds: u64,
    pub missed_source_rounds: u64,
    pub partition_dropped_rounds: u64,
    pub delivered_legitimate_rounds: u64,
    pub pending_legitimate_rounds_at_end: u64,
    pub timely_verified_rounds: u64,
    pub late_or_withheld_rounds: u64,
    pub periodic_checks_without_a_claim: u64,
    pub injected_tamper_attempts: u64,
    pub injected_replay_attempts: u64,
    pub authentic_false_reading_rounds: u64,
    pub model_classification_disagreements: u64,
}

#[derive(Debug, Serialize)]
pub struct Simulation {
    pub simulation_schema_version: u16,
    pub data_kind: String,
    pub config: Config,
    pub summary: SimulationSummary,
    pub samples: Vec<Sample>,
    pub frames: Vec<Frame>,
    pub trace: Vec<Trace>,
    pub workload: Workload,
    pub verification: ReplayReport,
}

#[derive(Clone)]
enum PacketKind {
    Legitimate,
    Tamper,
    Replay,
}
#[derive(Clone)]
struct Packet {
    bundle: Bundle,
    expected_event: String,
    expected_nonce: String,
    sequence: u64,
    captured: u64,
    kind: PacketKind,
}

struct Rng(u64);
impl Rng {
    fn noise(&mut self) -> i32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 32) % 11) as i32 - 5
    }
}

fn phase(c: &Config, t: u64) -> u64 {
    t * 100 / c.duration_ms
}
fn has(c: &Config, s: Scenario) -> bool {
    c.scenario == s || c.scenario == Scenario::Mixed
}
fn source_online(c: &Config, t: u64) -> bool {
    !(has(c, Scenario::Dropout) && (20..35).contains(&phase(c, t)))
}
fn connected(c: &Config, t: u64) -> bool {
    !(has(c, Scenario::Partition) && (45..60).contains(&phase(c, t)))
}
fn environment(c: &Config, t: u64) -> (u32, u32) {
    let p = phase(c, t);
    let co2 = if p <= 40 {
        600 + p * 10
    } else if p <= 70 {
        1000
    } else {
        1000 - (p - 70) * 12
    };
    (co2 as u32, if (40..60).contains(&p) { 20 } else { 500 })
}

fn bundle(template: &Bundle, policy: &Policy, config: &Config, s: &Sample) -> Bundle {
    let mut b = template.clone();
    let event = format!("sim-{:016x}-{:04}", config.seed, s.sequence);
    let nonce =
        digest(format!("veri-adam/simulation/v1/{}/{}", config.seed, s.sequence).as_bytes());
    for (i, m) in b.measurements.iter_mut().enumerate() {
        m.payload.event_id = event.clone();
        m.payload.nonce = nonce.clone();
        m.payload.sequence = s.sequence;
        m.payload.captured_at_ms = s.captured_at_ms;
        m.payload.policy_hash = policy.id();
        m.payload.co2_ppm = if i == 0 {
            s.measured_a_ppm.expect("only complete rounds are signed")
        } else {
            s.measured_b_ppm
        };
        m.payload.light_millilux = s.model_light_lux * 1000;
        m.signature = sign(&m.payload.canonical_bytes(), &[(i + 1) as u8; 32]);
    }
    let d = &mut b.decision.payload;
    d.event_id = event;
    d.nonce = nonce;
    d.policy_hash = policy.id();
    d.measurement_ids = b.measurements.iter().map(|m| m.payload.id()).collect();
    d.measurement_ids.sort();
    d.advisory = u64::from(s.measured_a_ppm.unwrap()) + u64::from(s.measured_b_ppm)
        >= u64::from(policy.threshold_ppm) * 2;
    b.decision.signature = sign(&d.canonical_bytes(), &[6; 32]);
    for (i, v) in b.ballots.iter_mut().enumerate() {
        v.payload.event_id = d.event_id.clone();
        v.payload.nonce = d.nonce.clone();
        v.payload.decision_hash = d.id();
        v.payload.advisory = d.advisory;
        v.signature = sign(&v.payload.canonical_bytes(), &[(i + 3) as u8; 32]);
    }
    b
}

fn trace(
    time: u64,
    kind: &str,
    detail: &str,
    sequence: Option<u64>,
    check_index: Option<usize>,
) -> Trace {
    Trace {
        time_ms: time,
        kind: kind.into(),
        detail: detail.into(),
        sequence,
        check_index,
    }
}
fn append_check(
    steps: &mut Vec<Step>,
    trust: (&Policy, &Registry),
    packet: &Packet,
    now: u64,
    heads: &BTreeMap<String, String>,
    latest_sequence: u64,
    label: &str,
) -> Outcome {
    let (p, r) = trust;
    // Model ground truth, not the production or reference checker, supplies the expectation.
    let expected = match packet.kind {
        PacketKind::Tamper | PacketKind::Replay => Outcome::Rejected,
        PacketKind::Legitimate if now >= packet.captured + p.ttl_ms => Outcome::Expired,
        PacketKind::Legitimate if packet.sequence < latest_sequence => Outcome::Unconfirmed,
        PacketKind::Legitimate => Outcome::VerifiedCurrent,
    };
    steps.push(Step {
        label: label.into(),
        reset_cache: false,
        policy: p.clone(),
        registry: r.clone(),
        request: Request {
            event_id: packet.expected_event.clone(),
            nonce: packet.expected_nonce.clone(),
            now_ms: now,
            kind: CheckKind::Current,
            heads: heads.clone(),
        },
        bundle: packet.bundle.clone(),
        expected,
    });
    expected
}

pub fn run(config: Config) -> Result<Simulation, String> {
    config.validate()?;
    let (mut policy, registry, _, template) = demo::fixture(CrewMode::NominalThree, [900, 850]);
    policy.ttl_ms = config.evidence_ttl_ms;
    let mut rng = Rng(config.seed);
    let mut summary = SimulationSummary::default();
    let mut samples = Vec::new();
    let mut schedule: BTreeMap<u64, Vec<Packet>> = BTreeMap::new();
    let mut trace_log = Vec::new();
    let mut first_packet: Option<Packet> = None;
    let mut replay_injected = false;
    // Stop captures before the horizon; report any delayed deliveries beyond it as pending.
    for (i, t) in (0..config.duration_ms)
        .step_by(config.sample_interval_ms as usize)
        .enumerate()
    {
        summary.attempted_rounds += 1;
        let (co2, light) = environment(&config, t);
        let biased =
            config.scenario == Scenario::FalseReading && (15..35).contains(&phase(&config, t));
        let a = (co2 as i32 - 10 + rng.noise() + if biased { 400 } else { 0 }) as u32;
        let b = (co2 as i32 + 10 + rng.noise()) as u32;
        let sample = Sample {
            captured_at_ms: t,
            sequence: i as u64 + 1,
            model_co2_ppm: co2,
            model_light_lux: light,
            measured_a_ppm: source_online(&config, t).then_some(a),
            measured_b_ppm: b,
            authentic_false_reading: biased,
        };
        samples.push(sample.clone());
        if sample.measured_a_ppm.is_none() {
            summary.missed_source_rounds += 1;
            trace_log.push(trace(
                t,
                "source-dropout",
                "Pi A produced no sample; no event is fabricated from an old reading",
                Some(sample.sequence),
                None,
            ));
            continue;
        }
        summary.created_rounds += 1;
        summary.authentic_false_reading_rounds += u64::from(biased);
        let signed = bundle(&template, &policy, &config, &sample);
        summary.model_classification_disagreements +=
            u64::from(signed.decision.payload.advisory != (co2 >= policy.threshold_ppm));
        let packet = Packet {
            expected_event: signed.decision.payload.event_id.clone(),
            expected_nonce: signed.decision.payload.nonce.clone(),
            bundle: signed,
            sequence: sample.sequence,
            captured: t,
            kind: PacketKind::Legitimate,
        };
        if first_packet.is_none() {
            first_packet = Some(packet.clone());
        }
        trace_log.push(trace(
            t,
            "sample",
            "Both logical nodes captured and signed a new event round",
            Some(packet.sequence),
            None,
        ));
        if !connected(&config, t) {
            summary.partition_dropped_rounds += 1;
            trace_log.push(trace(
                t,
                "partition-drop",
                "The complete claim packet is lost on the simulated link",
                Some(packet.sequence),
                None,
            ));
        } else {
            let delayed = has(&config, Scenario::Delay) && (65..80).contains(&phase(&config, t));
            let delay = if delayed {
                if packet.sequence.is_multiple_of(2) {
                    config.sample_interval_ms + config.transport_delay_ms + 1_000
                } else {
                    config.evidence_ttl_ms + config.transport_delay_ms + 1_000
                }
            } else {
                config.transport_delay_ms
            };
            let arrival = t + delay;
            if arrival > config.duration_ms {
                summary.pending_legitimate_rounds_at_end += 1;
            } else {
                schedule.entry(arrival).or_default().push(packet.clone());
            }
            if delayed {
                trace_log.push(trace(
                    t,
                    "network-delay",
                    "A claim is delayed; later rounds may arrive first",
                    Some(packet.sequence),
                    None,
                ));
            }
            if has(&config, Scenario::Tamper) && i == 1 {
                let mut attacked = packet.clone();
                attacked.kind = PacketKind::Tamper;
                attacked.bundle.measurements[0].payload.co2_ppm += 1;
                let at = t + config.transport_delay_ms + 100;
                if at <= config.duration_ms {
                    schedule.entry(at).or_default().push(attacked);
                    summary.injected_tamper_attempts += 1;
                }
            }
        }
        if has(&config, Scenario::Replay)
            && !replay_injected
            && phase(&config, t) >= 75
            && let Some(old) = &first_packet
        {
            let mut replay = old.clone();
            replay.kind = PacketKind::Replay;
            replay.expected_event = packet.expected_event.clone();
            replay.expected_nonce = packet.expected_nonce.clone();
            let at = t + config.transport_delay_ms + 100;
            if at <= config.duration_ms {
                schedule.entry(at).or_default().push(replay);
                summary.injected_replay_attempts += 1;
            }
            replay_injected = true;
        }
    }
    let mut times: BTreeSet<u64> = schedule.keys().copied().collect();
    for t in (0..=config.duration_ms).step_by(config.check_interval_ms as usize) {
        times.insert(t);
    }
    let mut frames = Vec::new();
    let mut steps = Vec::new();
    let mut heads = BTreeMap::new();
    let mut latest_sequence = 0;
    let mut active: Option<Packet> = None;
    for now in times {
        if let Some(packets) = schedule.remove(&now) {
            for packet in packets {
                if matches!(packet.kind, PacketKind::Legitimate) {
                    summary.delivered_legitimate_rounds += 1;
                    // Trusted checkpoint ground truth advances only for authentic, newer rounds.
                    // It is not inferred from a candidate verifier's accept/reject result.
                    if packet.sequence > latest_sequence {
                        heads = packet
                            .bundle
                            .measurements
                            .iter()
                            .map(|m| (m.payload.signer_id.clone(), m.payload.id()))
                            .collect();
                        latest_sequence = packet.sequence;
                        active = Some(packet.clone());
                    }
                }
                let label = format!("arrival-{}-{now}", packet.sequence);
                let index = steps.len();
                let outcome = append_check(
                    &mut steps,
                    (&policy, &registry),
                    &packet,
                    now,
                    &heads,
                    latest_sequence,
                    &label,
                );
                let kind = match packet.kind {
                    PacketKind::Legitimate => "delivery",
                    PacketKind::Tamper => "tamper",
                    PacketKind::Replay => "replay",
                };
                trace_log.push(trace(
                    now,
                    kind,
                    &format!(
                        "Expected {outcome:?}; virtual delivery age {} ms",
                        now - packet.captured
                    ),
                    Some(packet.sequence),
                    Some(index),
                ));
                if matches!(packet.kind, PacketKind::Legitimate)
                    && outcome == Outcome::VerifiedCurrent
                    && now - packet.captured <= config.deadline_ms
                {
                    summary.timely_verified_rounds += 1;
                }
            }
        }
        if now.is_multiple_of(config.check_interval_ms) {
            let (co2, light) = environment(&config, now);
            frames.push(Frame {
                time_ms: now,
                model_co2_ppm: co2,
                model_light_lux: light,
                pi_a_online: source_online(&config, now),
                network_connected: connected(&config, now),
            });
            if let Some(packet) = &active {
                let index = steps.len();
                let label = format!("periodic-{now}");
                let outcome = append_check(
                    &mut steps,
                    (&policy, &registry),
                    packet,
                    now,
                    &heads,
                    latest_sequence,
                    &label,
                );
                trace_log.push(trace(
                    now,
                    "check",
                    &format!("Expected {outcome:?}; active round {}", packet.sequence),
                    Some(packet.sequence),
                    Some(index),
                ));
            } else {
                summary.periodic_checks_without_a_claim += 1;
                trace_log.push(trace(
                    now,
                    "waiting",
                    "No authenticated claim has arrived yet",
                    None,
                    None,
                ));
            }
        }
    }
    summary.late_or_withheld_rounds = summary.attempted_rounds - summary.timely_verified_rounds;
    trace_log.sort_by_key(|t| t.time_ms);
    let workload = Workload {
        version: VERSION,
        label: format!("simulation-{:?}-seed-{}", config.scenario, config.seed),
        data_kind: "synthetic".into(),
        cache_config: veri_adam_cache::Config::default(),
        steps,
    };
    if serde_json::to_vec_pretty(&workload)
        .map_err(|e| e.to_string())?
        .len() as u64
        + 1
        > MAX_FILE_BYTES
    {
        return Err(
            "generated workload exceeds the 1 MiB replay limit; reduce duration or check frequency"
                .into(),
        );
    }
    let verification = veri_adam_replay::run(&workload)?;
    Ok(Simulation {
        simulation_schema_version: 1,
        data_kind: "synthetic".into(),
        config,
        summary,
        samples,
        frames,
        trace: trace_log,
        workload,
        verification,
    })
}

pub mod viewer;
