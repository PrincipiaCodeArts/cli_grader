use crate::grader::score::Mode as GradingMode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct GradingSection {
    pub mode: GradingMode,
}
