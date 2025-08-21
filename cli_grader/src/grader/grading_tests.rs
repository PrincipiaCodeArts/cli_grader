pub(crate) mod unit_test;
use crate::grader::grading_tests::unit_test::{UnitTests, UnitTestsResult};
use crate::grader::score::{GradingMode, Score};

/// This is the interface between the grader and the assessment modalities.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GradingTests {
    /// Modality in which only one program is tested against multiple simple test cases.
    UnitTests(UnitTests),
    // integration tests
    // performance tests
}
impl GradingTests {
    pub fn run(&self, grading_mode: GradingMode) -> GradindTestsResult {
        match self {
            GradingTests::UnitTests(unit_test) => {
                GradindTestsResult::UnitTests(unit_test.run(grading_mode))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GradindTestsResult {
    UnitTests(UnitTestsResult),
}

impl GradindTestsResult {
    pub fn score(&self) -> Score {
        match self {
            GradindTestsResult::UnitTests(r) => r.score(),
        }
    }
}
