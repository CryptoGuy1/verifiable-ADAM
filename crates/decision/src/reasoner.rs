use crate::*;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn rule_recommendation(c: &ReasoningContext, confidence: f64) -> Recommendation {
    let class = c.fusion.threshold_class;
    let alignment = match class {
        Classification::Normal => &c.normal_alignment,
        Classification::Anomaly => &c.anomaly_alignment,
    };
    Recommendation {
        classification: class,
        confidence,
        severity: alignment.severity,
        reasoning: format!(
            "The registered weighted concentration is {:.6} ppm against the experimental {} ppm threshold.",
            c.fusion.fused_ppm, c.threshold_ppm
        ),
        recommended_action: alignment.action.clone(),
        contributing_factors: vec!["registered_weighted_concentration".into()],
        requires_human_review: confidence < 0.6
            || c.fusion.dispersion_ppm > c.fusion.fused_ppm / 2.0
            || alignment.severity == Severity::Critical,
    }
}
pub struct RuleReasoner {
    pub confidence: f64,
}
impl Reasoner for RuleReasoner {
    fn infer(
        &mut self,
        c: &ReasoningContext,
        _repair: Option<&str>,
        _remaining: u64,
    ) -> Result<ReasonerResponse, String> {
        let recommendation = rule_recommendation(c, self.confidence);
        recommendation.validate()?;
        Ok(ReasonerResponse {
            backend: "configured-rule-reasoner".into(),
            raw_response: serde_json::to_string(&recommendation).map_err(|e| e.to_string())?,
            metadata: serde_json::json!({"model_inference":false,"failure_fallback":false}),
        })
    }
}
pub struct OllamaReasoner {
    pub model: String,
    pub adapter: std::path::PathBuf,
}
impl OllamaReasoner {
    pub fn prepare(&self) -> Result<serde_json::Value, String> {
        let output = Command::new("python3")
            .arg(&self.adapter)
            .arg(&self.model)
            .arg("--warmup")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Model preparation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
    }
}
impl Reasoner for OllamaReasoner {
    fn infer(
        &mut self,
        context: &ReasoningContext,
        repair_of: Option<&str>,
        remaining_ms: u64,
    ) -> Result<ReasonerResponse, String> {
        if remaining_ms == 0 {
            return Err("no inference budget remains".into());
        }
        let mut child = Command::new("python3")
            .env(
                "VERI_ADAM_VERIFIER",
                std::env::current_exe().map_err(|e| e.to_string())?,
            )
            .arg(&self.adapter)
            .arg(&self.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let payload=serde_json::to_vec(&serde_json::json!({"context":context,"repair_of":repair_of,"remaining_ms":remaining_ms})).map_err(|e|e.to_string())?;
        child
            .stdin
            .take()
            .ok_or("adapter input unavailable")?
            .write_all(&payload)
            .map_err(|e| e.to_string())?;
        // Drain pipes concurrently: large explanations must not block the process before exit.
        let stdout = child.stdout.take().ok_or("adapter stdout unavailable")?;
        let stderr = child.stderr.take().ok_or("adapter stderr unavailable")?;
        let out = thread::spawn(move || {
            let mut b = vec![];
            stdout.take(131073).read_to_end(&mut b).map(|_| b)
        });
        let err = thread::spawn(move || {
            let mut b = vec![];
            stderr.take(8193).read_to_end(&mut b).map(|_| b)
        });
        let end = Instant::now() + Duration::from_millis(remaining_ms);
        let mut timeout = false;
        while child.try_wait().map_err(|e| e.to_string())?.is_none() {
            if Instant::now() >= end {
                timeout = true;
                let _ = child.kill();
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let bytes = out
            .join()
            .map_err(|_| "stdout reader failed")?
            .map_err(|e| e.to_string())?;
        let errors = err
            .join()
            .map_err(|_| "stderr reader failed")?
            .map_err(|e| e.to_string())?;
        if timeout {
            return Err("model response exceeded remaining event budget".into());
        }
        if !status.success() {
            return Err(String::from_utf8_lossy(&errors).chars().take(600).collect());
        }
        if bytes.len() > 131072 {
            return Err("model adapter response exceeded bound".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}
