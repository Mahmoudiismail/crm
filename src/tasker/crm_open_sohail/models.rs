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

#[derive(Debug, Clone)]
pub struct TeamMappingInfo {
    pub owner_name: String,
    pub owner_email: String,
    pub is_shared: bool,
}

#[derive(Debug)]
pub struct EnrichedRow {
    pub team: String,
    pub closed: f64,
    pub open: f64,
    pub perc_closed: String,
    pub perc_open: String,
    pub grand_total: f64,
    pub oul: String,
}

#[derive(Debug)]
pub struct EnrichedDataset {
    pub branch: String,
    pub month: String,
    pub data: Vec<EnrichedRow>,
}
