use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ExtractedPivotRow {
    #[serde(rename = "team")]
    pub team: String,
    #[serde(rename = "closed")]
    pub closed: f64,
    #[serde(rename = "open")]
    pub open: f64,
    #[serde(rename = "% of closed")]
    pub perc_closed: String,
    #[serde(rename = "% of open")]
    pub perc_open: String,
    #[serde(rename = "Grand Total")]
    pub grand_total: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExtractedSlicerDataset {
    pub branch: String,
    pub month: String,
    pub data: Vec<ExtractedPivotRow>,
}
