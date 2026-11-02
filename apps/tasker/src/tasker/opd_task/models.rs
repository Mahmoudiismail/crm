use chrono::NaiveDate;
use std::collections::HashMap;

/// Represents a row of parsed or merged CUS data.
#[derive(Clone, Debug, PartialEq)]
pub struct CusRow {
    pub ksa_time: NaiveDate,
    pub day: String,
    pub times: HashMap<String, String>,
    pub others: HashMap<String, String>,
}
