use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReportOutput {
    Txt,
    #[default]
    Stdout,
}
