pub mod unit_test;
use crate::grader::assessment_modalities::unit_test::{UnitTest, UnitTestResult};
use crate::grader::score::{Mode, Score};

/// This is the interface between the grader and the assessment modalities.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Assessment {
    /// Modality in which only one program is tested against multiple simple test cases.
    UnitTest(UnitTest),
    // integration tests
    // performance tests
}
impl Assessment {
    pub fn run(&self, grading_mode: Mode) -> AssessmentResult {
        match self {
            Assessment::UnitTest(unit_test) => {
                AssessmentResult::UnitTest(unit_test.run(grading_mode))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AssessmentResult {
    UnitTest(UnitTestResult),
}

impl AssessmentResult {
    pub fn score(&self) -> Score {
        match self {
            AssessmentResult::UnitTest(r) => r.score(),
        }
    }
}
