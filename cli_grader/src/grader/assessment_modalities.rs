pub mod unit_test;
use crate::grader::assessment_modalities::unit_test::{UnitTest, UnitTestResult};
use crate::grader::score::{GradingMode, Score};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Test {
    UnitTest(UnitTest),
    // integration tests
    // performance tests
}
impl Test {
    pub fn run_tests(&self, grading_mode: GradingMode) -> TestResult {
        match self {
            Test::UnitTest(unit_test) => TestResult::UnitTest(unit_test.run_tests(grading_mode)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TestResult {
    UnitTest(UnitTestResult),
}

impl TestResult {
    pub fn score(&self) -> Score {
        match self {
            TestResult::UnitTest(r) => r.score(),
        }
    }
}

// TODO (checkpoint):continue with the organization of the modules.
// It is still necessary to check more thoroughly the inner part of assessment modalities.
