//! Explicitly synthetic numerical and application-policy fixtures, not client defaults.
use crate::{calibration::*, *};
use std::collections::BTreeSet;

pub fn calibration_data() -> CalibrationData {
    CalibrationData{version:1,provenance:"Synthetic paired reference fixture; not SCD40 calibration or historical methane calibration.".into(),synthetic:true,
        series:[("sensor-a",1), ("sensor-b",2)].into_iter().map(|(id,multiplier)|CalibrationSeries{source_id:id.into(),pairs:(0..500).map(|j|{
            let reference=600+(j%10)*50;
            let residual=([-2,-1,0,1,2][j as usize%5])*multiplier;
            PairedObservation{reference_ppm:reference as u32,sensor_ppm:(reference+residual) as u32}
        }).collect()}).collect()}
}
pub fn policy() -> ApplicationPolicy {
    ApplicationPolicy{version:1,provenance:"Development application policy. Numerical governance checks follow ADAM sections 3.3-3.5; action names/alignment are explicit fixture choices, not a recovered client action table.".into(),
        threshold_ppm:800,event_budget_ms:30000,evidence_ttl_ms:30000,competition_window_ms:1000,
        confidence_floor:0.35,high_concentration_ppm:5000,high_concentration_source:GovernanceConcentration::TriggerReading,
        permitted_actions:BTreeSet::from(["ContinueMonitoring".into(),"VentilationAdvisory".into(),"RequestHumanReview".into()]),
        passive_actions:BTreeSet::from(["ContinueMonitoring".into()]),
        normal_alignment:Alignment{severity:Severity::None,action:"ContinueMonitoring".into()},
        anomaly_alignment:Alignment{severity:Severity::Low,action:"VentilationAdvisory".into()},fallback_confidence:0.5}
}
pub fn event(id: &str, start: u64, values: &[(&str, u32)]) -> EventInput {
    EventInput {
        event_id: id.into(),
        session_id: "synthetic-session".into(),
        location_id: "synthetic-room".into(),
        nonce: veri_adam_protocol::digest(id.as_bytes()),
        started_at_ms: start,
        trigger_source: values.first().map(|v| v.0).unwrap_or("sensor-a").into(),
        readings: values
            .iter()
            .map(|(source, ppm)| Reading {
                evidence_id: veri_adam_protocol::digest(format!("{id}/{source}/{ppm}").as_bytes()),
                source_id: (*source).into(),
                sequence: start + 1,
                captured_at_ms: start,
                co2_ppm: *ppm,
                data_kind: "synthetic".into(),
            })
            .collect(),
        temporal_window: vec![],
        availability: CrewAvailability {
            sensor: true,
            aggregator: true,
            decision: true,
            coordinator: true,
        },
        missing_ballots: BTreeSet::new(),
        competing: vec![],
    }
}
