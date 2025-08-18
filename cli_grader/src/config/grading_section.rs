use crate::grader::score::GradingMode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct GradingSection {
    pub mode: GradingMode,
}
