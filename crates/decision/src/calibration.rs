use crate::{Reading, bounded_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use veri_adam_protocol::digest;

pub const WEIGHT_SCALE: u32 = 1_000_000;
pub const MAX_PPM: u32 = 40_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedObservation {
    pub sensor_ppm: u32,
    pub reference_ppm: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationSeries {
    pub source_id: String,
    pub pairs: Vec<PairedObservation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationData {
    pub version: u64,
    pub provenance: String,
    pub synthetic: bool,
    pub series: Vec<CalibrationSeries>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibratedWeight {
    pub source_id: String,
    pub paired_count: usize,
    pub residual_mean_ppm: f64,
    pub residual_variance_ppm2: f64,
    pub normalized_inverse_variance: f64,
    pub weight_units: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationProfile {
    pub version: u64,
    pub input_sha256: String,
    pub provenance: String,
    pub synthetic: bool,
    pub scale: u32,
    pub weights: Vec<CalibratedWeight>,
}
impl CalibrationProfile {
    pub fn id(&self) -> String {
        digest(&serde_json::to_vec(self).expect("finite calibrated profile"))
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version == 0
            || self.scale != WEIGHT_SCALE
            || self.weights.is_empty()
            || self.weights.len() > 32
            || self.provenance.trim().is_empty()
        {
            return Err("invalid calibration profile".into());
        }
        let mut sources = BTreeSet::new();
        let mut total = 0u64;
        for w in &self.weights {
            bounded_id(&w.source_id)?;
            if !sources.insert(&w.source_id)
                || w.weight_units == 0
                || w.paired_count < 2
                || !w.residual_variance_ppm2.is_finite()
                || w.residual_variance_ppm2 <= 0.0
                || !w.residual_mean_ppm.is_finite()
                || !w.normalized_inverse_variance.is_finite()
                || !(0.0..=1.0).contains(&w.normalized_inverse_variance)
            {
                return Err("invalid calibration weight".into());
            }
            total += u64::from(w.weight_units);
        }
        if total != u64::from(self.scale) {
            return Err("calibration weights do not sum to scale".into());
        }
        veri_adam_protocol::decode::<32>(&self.input_sha256)?;
        Ok(())
    }
}

pub fn calibrate(data: &CalibrationData) -> Result<CalibrationProfile, String> {
    if data.version == 0
        || data.provenance.trim().is_empty()
        || data.provenance.len() > 1200
        || data.series.is_empty()
        || data.series.len() > 32
    {
        return Err("invalid calibration dataset".into());
    }
    let mut seen = BTreeSet::new();
    let mut weights = Vec::new();
    let mut inverse = Vec::new();
    for series in &data.series {
        bounded_id(&series.source_id)?;
        if !seen.insert(&series.source_id) || !(2..=10_000).contains(&series.pairs.len()) {
            return Err(
                "calibration needs unique sources and 2..10000 paired observations per source"
                    .into(),
            );
        }
        let mut sum = 0i128;
        let mut squares = 0i128;
        for p in &series.pairs {
            if p.sensor_ppm > MAX_PPM || p.reference_ppm > MAX_PPM {
                return Err("calibration ppm outside protocol bounds".into());
            }
            let r = i128::from(p.sensor_ppm) - i128::from(p.reference_ppm);
            sum += r;
            squares += r * r;
        }
        let n = series.pairs.len() as i128;
        // Exact integer moment calculation avoids catastrophic cancellation from sensor bias.
        let numerator = n * squares - sum * sum;
        if numerator <= 0 {
            return Err("zero residual variance cannot define inverse-variance weights".into());
        }
        let variance = numerator as f64 / (n * (n - 1)) as f64;
        inverse.push(1.0 / variance);
        weights.push(CalibratedWeight {
            source_id: series.source_id.clone(),
            paired_count: n as usize,
            residual_mean_ppm: sum as f64 / n as f64,
            residual_variance_ppm2: variance,
            normalized_inverse_variance: 0.0,
            weight_units: 0,
        });
    }
    let total: f64 = inverse.iter().sum();
    let mut fractions = Vec::new();
    let mut allocated = 0u32;
    for (i, (weight, inv)) in weights.iter_mut().zip(inverse).enumerate() {
        weight.normalized_inverse_variance = inv / total;
        let scaled = weight.normalized_inverse_variance * f64::from(WEIGHT_SCALE);
        weight.weight_units = scaled.floor() as u32;
        allocated += weight.weight_units;
        fractions.push((i, scaled - scaled.floor()));
    }
    if allocated > WEIGHT_SCALE || (WEIGHT_SCALE - allocated) as usize > weights.len() {
        return Err("weight quantization failed".into());
    }
    fractions.sort_by(|(a, fa), (b, fb)| {
        fb.total_cmp(fa)
            .then_with(|| weights[*a].source_id.cmp(&weights[*b].source_id))
    });
    for (i, _) in fractions
        .into_iter()
        .take((WEIGHT_SCALE - allocated) as usize)
    {
        weights[i].weight_units += 1;
    }
    weights.sort_by(|a, b| a.source_id.cmp(&b.source_id));
    let profile = CalibrationProfile {
        version: data.version,
        input_sha256: digest(&serde_json::to_vec(data).map_err(|e| e.to_string())?),
        provenance: data.provenance.clone(),
        synthetic: data.synthetic,
        scale: WEIGHT_SCALE,
        weights,
    };
    profile.validate()?;
    Ok(profile)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fusion {
    pub calibration_hash: String,
    pub contributing_sources: Vec<String>,
    pub weighted_sum: u64,
    pub weight_sum: u64,
    pub fused_ppm: f64,
    pub dispersion_ppm: f64,
    pub unquantized_fused_ppm: f64,
    pub quantization_delta_ppm: f64,
    pub threshold_class: crate::Classification,
}
pub fn fuse(
    readings: &[Reading],
    profile: &CalibrationProfile,
    threshold: u32,
) -> Result<Fusion, String> {
    profile.validate()?;
    if readings.is_empty() || readings.len() > 32 || threshold > MAX_PPM {
        return Err("invalid fusion inputs".into());
    }
    let mut seen = BTreeSet::new();
    let mut weighted_sum = 0u64;
    let mut weight_sum = 0u64;
    let mut values = Vec::new();
    let mut original_sum = 0.0;
    let mut original_weight = 0.0;
    for r in readings {
        r.validate()?;
        if !seen.insert(r.source_id.clone()) {
            return Err("duplicate fusion source".into());
        }
        let w = profile
            .weights
            .iter()
            .find(|w| w.source_id == r.source_id)
            .ok_or("source has no registered calibration weight")?;
        weighted_sum += u64::from(w.weight_units) * u64::from(r.co2_ppm);
        weight_sum += u64::from(w.weight_units);
        values.push((f64::from(r.co2_ppm), f64::from(w.weight_units)));
        original_sum += w.normalized_inverse_variance * f64::from(r.co2_ppm);
        original_weight += w.normalized_inverse_variance;
    }
    let mean = weighted_sum as f64 / weight_sum as f64;
    let variance = values
        .iter()
        .map(|(v, w)| w * (v - mean).powi(2))
        .sum::<f64>()
        / weight_sum as f64;
    let original = original_sum / original_weight;
    let mut contributing_sources: Vec<_> = seen.into_iter().collect();
    contributing_sources.sort();
    Ok(Fusion {
        calibration_hash: profile.id(),
        contributing_sources,
        weighted_sum,
        weight_sum,
        fused_ppm: mean,
        dispersion_ppm: variance.sqrt(),
        unquantized_fused_ppm: original,
        quantization_delta_ppm: mean - original,
        threshold_class: crate::Classification::from_anomaly(
            weighted_sum >= u64::from(threshold) * weight_sum,
        ),
    })
}
