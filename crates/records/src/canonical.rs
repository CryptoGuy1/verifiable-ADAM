//! Deterministic, typed-tree binary encoding, separate from legacy v1 bytes.
use serde::Serialize;
use serde_json::Value;
use veri_adam_protocol::digest;

pub fn bytes<T: Serialize>(kind: &str, value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    string(&mut out, &format!("veri-adam/{kind}/v2"));
    encode(
        &mut out,
        &serde_json::to_value(value).expect("finite validated wire value"),
    );
    out
}
pub fn id<T: Serialize>(kind: &str, value: &T) -> String {
    digest(&bytes(kind, value))
}
fn string(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u32).to_be_bytes());
    out.extend(s.as_bytes());
}
fn encode(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.push(0),
        Value::Bool(b) => out.push(if *b { 2 } else { 1 }),
        Value::Number(n) => {
            if let Some(n) = n.as_u64() {
                out.push(3);
                out.extend(n.to_be_bytes());
            } else if let Some(n) = n.as_i64() {
                out.push(4);
                out.extend(n.to_be_bytes());
            } else {
                out.push(5);
                out.extend(n.as_f64().expect("finite number").to_bits().to_be_bytes());
            }
        }
        Value::String(s) => {
            out.push(6);
            string(out, s);
        }
        Value::Array(a) => {
            out.push(7);
            out.extend((a.len() as u32).to_be_bytes());
            for v in a {
                encode(out, v);
            }
        }
        Value::Object(m) => {
            out.push(8);
            out.extend((m.len() as u32).to_be_bytes());
            let sorted: std::collections::BTreeMap<_, _> = m.iter().collect();
            for (k, v) in sorted {
                string(out, k);
                encode(out, v);
            }
        }
    }
}
pub fn hidden<T: Serialize>(kind: &str, salt: &str, value: &T) -> String {
    id(kind, &(salt, value))
}
