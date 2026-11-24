//! Version 1 experimental records. JSON is transport; explicit binary encoding is signed.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: u16 = 1;
pub const MAX_RECORDS: usize = 32;
pub const MAX_FILE_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Sensor,
    Voter,
    Coordinator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub signer_id: String,
    pub node_id: String,
    pub role: Role,
    pub public_key: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub version: u64,
    pub members: Vec<Member>,
}

impl Registry {
    pub fn member(&self, id: &str, role: Role) -> Option<&Member> {
        self.members
            .iter()
            .find(|m| m.signer_id == id && m.role == role && !m.revoked)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0 || self.members.is_empty() || self.members.len() > MAX_RECORDS {
            return Err("invalid registry size/version".into());
        }
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for m in &self.members {
            identifier(&m.signer_id)?;
            identifier(&m.node_id)?;
            if !ids.insert(&m.signer_id) || !keys.insert(&m.public_key) {
                return Err("duplicate registry identity/key".into());
            }
            let key = VerifyingKey::from_bytes(&decode::<32>(&m.public_key)?)
                .map_err(|_| "invalid public key")?;
            if key.is_weak() {
                return Err("weak public key".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrewMode {
    NominalThree,
    DegradedTwo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub protocol_version: u16,
    pub policy_version: u64,
    pub threshold_ppm: u32,
    pub ttl_ms: u64,
    pub sources: Vec<String>,
    pub voters: Vec<String>,
    pub coordinator: String,
    pub crew_mode: CrewMode,
}

impl Policy {
    pub fn validate(&self, registry: &Registry) -> Result<(), String> {
        registry.validate()?;
        if self.protocol_version != VERSION
            || self.policy_version == 0
            || self.ttl_ms == 0
            || self.threshold_ppm > 40_000
            || self.sources.is_empty()
            || self.sources.len() > MAX_RECORDS
        {
            return Err("invalid policy parameters".into());
        }
        if self.voters.len()
            != match self.crew_mode {
                CrewMode::NominalThree => 3,
                CrewMode::DegradedTwo => 2,
            }
        {
            return Err("crew mode requires exactly three or two eligible voters".into());
        }
        for (ids, role) in [(&self.sources, Role::Sensor), (&self.voters, Role::Voter)] {
            if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
                return Err("duplicate policy identity".into());
            }
            for id in ids {
                identifier(id)?;
                if registry.member(id, role.clone()).is_none() {
                    return Err("policy identity unauthorized/revoked".into());
                }
            }
        }
        identifier(&self.coordinator)?;
        if registry
            .member(&self.coordinator, Role::Coordinator)
            .is_none()
        {
            return Err("unauthorized coordinator".into());
        }
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new("veri-adam/policy/v1");
        e.u16(self.protocol_version);
        e.u64(self.policy_version);
        e.u32(self.threshold_ppm);
        e.u64(self.ttl_ms);
        e.strings(&self.sources);
        e.strings(&self.voters);
        e.string(&self.coordinator);
        e.u8(match self.crew_mode {
            CrewMode::NominalThree => 0,
            CrewMode::DegradedTwo => 1,
        });
        e.0
    }
    pub fn id(&self) -> String {
        digest(&self.canonical_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub protocol_version: u16,
    pub event_id: String,
    pub nonce: String,
    pub signer_id: String,
    pub sequence: u64,
    pub captured_at_ms: u64,
    pub co2_ppm: u32,
    pub temperature_millic: i32,
    pub humidity_millipercent: u32,
    pub light_millilux: u32,
    pub calibration_version: u64,
    pub policy_hash: String,
}

impl Measurement {
    pub fn validate(&self) -> Result<(), String> {
        context(self.protocol_version, &self.event_id, &self.nonce)?;
        identifier(&self.signer_id)?;
        decode::<32>(&self.policy_hash)?;
        if self.sequence == 0
            || self.calibration_version == 0
            || self.co2_ppm > 40_000
            || !(-40_000..=125_000).contains(&self.temperature_millic)
            || self.humidity_millipercent > 100_000
        {
            return Err("measurement outside protocol bounds".into());
        }
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new("veri-adam/measurement/v1");
        e.u16(self.protocol_version);
        e.string(&self.event_id);
        e.string(&self.nonce);
        e.string(&self.signer_id);
        e.u64(self.sequence);
        e.u64(self.captured_at_ms);
        e.u32(self.co2_ppm);
        e.0.extend(self.temperature_millic.to_be_bytes());
        e.u32(self.humidity_millipercent);
        e.u32(self.light_millilux);
        e.u64(self.calibration_version);
        e.string(&self.policy_hash);
        e.0
    }
    pub fn id(&self) -> String {
        digest(&self.canonical_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedMeasurement {
    pub payload: Measurement,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub protocol_version: u16,
    pub event_id: String,
    pub nonce: String,
    pub policy_hash: String,
    pub registry_version: u64,
    pub coordinator_id: String,
    pub measurement_ids: Vec<String>,
    pub advisory: bool,
}
impl Decision {
    pub fn validate(&self) -> Result<(), String> {
        context(self.protocol_version, &self.event_id, &self.nonce)?;
        identifier(&self.coordinator_id)?;
        decode::<32>(&self.policy_hash)?;
        if self.registry_version == 0
            || self.measurement_ids.is_empty()
            || self.measurement_ids.len() > MAX_RECORDS
            || !self.measurement_ids.windows(2).all(|w| w[0] < w[1])
        {
            return Err("decision references must be nonempty, unique and sorted".into());
        }
        for id in &self.measurement_ids {
            decode::<32>(id)?;
        }
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new("veri-adam/decision/v1");
        e.u16(self.protocol_version);
        e.string(&self.event_id);
        e.string(&self.nonce);
        e.string(&self.policy_hash);
        e.u64(self.registry_version);
        e.string(&self.coordinator_id);
        e.strings(&self.measurement_ids);
        e.u8(u8::from(self.advisory));
        e.0
    }
    pub fn id(&self) -> String {
        digest(&self.canonical_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedDecision {
    pub payload: Decision,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ballot {
    pub protocol_version: u16,
    pub event_id: String,
    pub nonce: String,
    pub decision_hash: String,
    pub voter_id: String,
    pub advisory: bool,
}
impl Ballot {
    pub fn validate(&self) -> Result<(), String> {
        context(self.protocol_version, &self.event_id, &self.nonce)?;
        identifier(&self.voter_id)?;
        decode::<32>(&self.decision_hash)?;
        Ok(())
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new("veri-adam/ballot/v1");
        e.u16(self.protocol_version);
        e.string(&self.event_id);
        e.string(&self.nonce);
        e.string(&self.decision_hash);
        e.string(&self.voter_id);
        e.u8(u8::from(self.advisory));
        e.0
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedBallot {
    pub payload: Ballot,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub decision: SignedDecision,
    pub measurements: Vec<SignedMeasurement>,
    pub ballots: Vec<SignedBallot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Current,
    Historical,
}

/// Trusted verifier input, NEVER taken from the event bundle itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub event_id: String,
    pub nonce: String,
    pub now_ms: u64,
    pub kind: CheckKind,
    pub heads: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    VerifiedCurrent,
    VerifiedHistorical,
    Expired,
    Unconfirmed,
    Rejected,
}

pub fn sign(bytes: &[u8], seed: &[u8; 32]) -> String {
    hex::encode(SigningKey::from_bytes(seed).sign(bytes).to_bytes())
}
pub fn public_key(seed: &[u8; 32]) -> String {
    hex::encode(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}
pub fn verify(bytes: &[u8], signature: &str, public_key: &str) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(&decode::<32>(public_key)?).map_err(|_| "invalid key")?;
    let sig = Signature::from_bytes(&decode::<64>(signature)?);
    key.verify_strict(bytes, &sig)
        .map_err(|_| "invalid signature".into())
}
pub fn validate_public_key(public_key: &str) -> Result<(), String> {
    let key =
        VerifyingKey::from_bytes(&decode::<32>(public_key)?).map_err(|_| "invalid public key")?;
    if key.is_weak() {
        Err("weak public key".into())
    } else {
        Ok(())
    }
}
pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn identifier(s: &str) -> Result<(), String> {
    if s.is_empty()
        || s.len() > 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    {
        Err("identifier must be 1..64 ASCII letters, digits, -_.:".into())
    } else {
        Ok(())
    }
}
pub fn decode<const N: usize>(s: &str) -> Result<[u8; N], String> {
    if s.len() != N * 2
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("expected fixed-width lowercase hexadecimal".into());
    }
    let bytes = hex::decode(s).map_err(|_| "invalid hex")?;
    bytes.try_into().map_err(|_| "wrong byte length".into())
}
pub fn context(version: u16, event: &str, nonce: &str) -> Result<(), String> {
    if version != VERSION {
        return Err("unsupported protocol version".into());
    }
    identifier(event)?;
    decode::<32>(nonce)?;
    Ok(())
}

struct Encoder(Vec<u8>);
impl Encoder {
    fn new(domain: &str) -> Self {
        let mut e = Self(Vec::new());
        e.string(domain);
        e
    }
    fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    fn u16(&mut self, x: u16) {
        self.0.extend(x.to_be_bytes());
    }
    fn u32(&mut self, x: u32) {
        self.0.extend(x.to_be_bytes());
    }
    fn u64(&mut self, x: u64) {
        self.0.extend(x.to_be_bytes());
    }
    fn string(&mut self, s: &str) {
        self.u32(s.len().try_into().expect("bounded protocol string"));
        self.0.extend(s.as_bytes());
    }
    fn strings(&mut self, ss: &[String]) {
        self.u32(ss.len().try_into().expect("bounded protocol array"));
        for s in ss {
            self.string(s);
        }
    }
}

pub mod demo;
