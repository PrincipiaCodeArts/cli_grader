use crate::grader::score::GradingMode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct GradingSection {
    mode: GradingMode,
}

impl GradingSection {
    pub fn new(mode: GradingMode) -> Self {
        Self { mode }
    }
    pub fn get_grading_mode(&self) -> GradingMode {
        self.mode
    }
}
