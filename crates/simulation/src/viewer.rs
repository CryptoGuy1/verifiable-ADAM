use super::Simulation;

pub fn html(simulation: &Simulation) -> Result<String, String> {
    let view = serde_json::json!({
        "config":simulation.config,"summary":simulation.summary,"samples":simulation.samples,
        "frames":simulation.frames,"trace":simulation.trace,"checks":simulation.verification.steps,
        "totals":simulation.verification.summary,"all_matched":simulation.verification.all_matched
    });
    let data = serde_json::to_string(&view)
        .map_err(|e| e.to_string())?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(include_str!("viewer.html").replace("__SIMULATION_DATA__", &data))
}
