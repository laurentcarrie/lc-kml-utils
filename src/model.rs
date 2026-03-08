use serde::Deserialize;

#[derive(Deserialize)]
pub struct PointDefinition {
    pub kml: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct ConcentricCircles {
    pub center: PointDefinition,
    pub name: String,
    pub v_radius: Vec<f64>,
    #[serde(default)]
    pub circle_on_top: bool,
}

#[derive(Deserialize)]
pub enum EChoice {
    ConcentricCircles(ConcentricCircles),
}

#[derive(Deserialize)]
pub struct InputData {
    pub choices: Vec<EChoice>,
}
