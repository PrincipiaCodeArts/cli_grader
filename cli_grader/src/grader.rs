mod assertions;

use assertions::{Assertion, AssertionResult};
use log::{debug, error};
use std::{
    env,
    io::{self},
    ops::{Add, AddAssign, Mul},
    process,
};

// Grader ------------------------------------------------------------------------
// TODO (optimization opportunity): replace `String` with &'a str for the string
// fields.

// Generic
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum GradingMode {
    Absolute,
    Weighted,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Score {
    AbsoluteScore(bool),
    WeightedScore { current: u32, max: u32 },
}

impl Score {
    fn default(grading_mode: GradingMode) -> Self {
        match grading_mode {
            GradingMode::Absolute => Self::AbsoluteScore(false),
            GradingMode::Weighted => Self::WeightedScore { current: 0, max: 0 },
        }
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Score::AbsoluteScore(b1), Score::AbsoluteScore(b2)) => *b1 = *b1 && b2,
            (
                Score::WeightedScore {
                    current: c1,
                    max: m1,
                },
                Score::WeightedScore {
                    current: c2,
                    max: m2,
                },
            ) => {
                *c1 = *c1 + c2;
                *m1 = *m1 + m2;
            }
            _ => panic!("unexpected addition between different scoring modes"),
        };
    }
}

impl Mul<u32> for Score {
    type Output = Score;

    fn mul(self, rhs: u32) -> Self::Output {
        match self {
            Score::WeightedScore { current: c, max: m } => Score::WeightedScore {
                current: c * rhs,
                max: m * rhs,
            },
            Score::AbsoluteScore(b) => Score::AbsoluteScore(b),
        }
    }
}

impl Add for Score {
    type Output = Score;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Score::AbsoluteScore(b1), Score::AbsoluteScore(b2)) => Self::AbsoluteScore(b1 && b2),
            (
                Score::WeightedScore {
                    current: c1,
                    max: m1,
                },
                Score::WeightedScore {
                    current: c2,
                    max: m2,
                },
            ) => Self::WeightedScore {
                current: c1 + c2,
                max: m1 + m2,
            },
            _ => panic!("unexpected addition between different scoring modes"),
        }
    }
}

// TODO (enhance): for now, a command will be represented as a string. Maybe, that can be
// changed to something more robust.
// TODO (improve): maybe there is a better way to represent filesystem paths
// other than `String`.
type Path = String;

// Input
// TODO (refactor): check the necessity to create a trait here.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct TargetProgram {
    name: String,
    path: Path,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ProgramUnitAssertions {
    name: String, // Default: `Unit testing <TargetProgram>`
    grading_mode: GradingMode,
    program: TargetProgram,
    assertions: Vec<Assertion>,
}
impl ProgramUnitAssertions {
    fn run_assertions(
        &self,
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        setup: Vec<String>,
        teardown: Vec<String>,
    ) -> io::Result<ProgramUnitAssertionResults> {
        let mut result = ProgramUnitAssertionResults::new(
            self.name.clone(),
            self.program.clone(),
            self.grading_mode,
        );
        let tmp_dir = env::temp_dir();
        for assertion in self.assertions.iter() {
            // execute setup
            for setup_cmd in setup.iter() {
                let mut setup_cmd = process::Command::new(setup_cmd);
                if !inherited_parent_envs {
                    setup_cmd.env_clear();
                }
                setup_cmd.current_dir(&tmp_dir);
                setup_cmd.envs(envs.clone());
                if let Err(err) = setup_cmd.output() {
                    error!("error while executing setup");
                    debug!("error: {err:?}");
                    return Err(err);
                }
            }

            // setup cmd
            let cmd = process::Command::new(&self.program.path);
            result.add_assertion_result(assertion.unsafe_assert_cmd(cmd));

            // execute teardown
            for teardown_cmd in teardown.iter() {
                let mut teardown_cmd = process::Command::new(teardown_cmd);
                if !inherited_parent_envs {
                    teardown_cmd.env_clear();
                }
                teardown_cmd.current_dir(&tmp_dir);
                teardown_cmd.envs(envs.clone());
                if let Err(err) = teardown_cmd.output() {
                    error!("error while executing teardown");
                    debug!("error: {err:?}");
                    return Err(err);
                }
            }
        }
        Ok(result)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ProgramUnitAssertionResults {
    name: String,
    program: TargetProgram,
    score: Score,
    assertion_results: Vec<AssertionResult>,
}

impl ProgramUnitAssertionResults {
    fn new(name: String, program: TargetProgram, grading_mode: GradingMode) -> Self {
        Self {
            name,
            program,
            score: Score::default(grading_mode),
            assertion_results: vec![],
        }
    }

    fn add_assertion_result(&mut self, assertion_result: AssertionResult) {
        self.score = match self.score {
            Score::AbsoluteScore(b) => {
                Score::AbsoluteScore(b && assertion_result.score() == assertion_result.max_score())
            }
            Score::WeightedScore { current: c, max: m } => Score::WeightedScore {
                current: c + assertion_result.score(),
                max: m + assertion_result.max_score(),
            },
        };
        self.assertion_results.push(assertion_result);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Tests {
    UnitTest {
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        setup: Vec<String>,
        teardown: Vec<String>,
        assertions_per_program: Vec<ProgramUnitAssertions>,
        grading_mode: GradingMode,
    },
    // integration tests
    // performance tests
}
impl Tests {
    fn run_tests(&self) -> TestResults {
        let mut score;
        match self {
            Tests::UnitTest {
                envs,
                grading_mode,
                inherited_parent_envs,
                setup,
                teardown,
                assertions_per_program,
            } => {
                score = Score::default(*grading_mode);
                let mut test_results = vec![];
                for program_unit_assertion in assertions_per_program {
                    let res = program_unit_assertion
                        .run_assertions(
                            envs.clone(),
                            *inherited_parent_envs,
                            setup.clone(),
                            teardown.clone(),
                        )
                        .expect("error during assertion");
                    score += res.score;
                    test_results.push(res);
                }
                return TestResults::UnitTestResults {
                    score,
                    assertion_per_program_results: test_results,
                };
            }
        }
    }
}

enum TestResults {
    UnitTestResults {
        score: Score,
        assertion_per_program_results: Vec<ProgramUnitAssertionResults>,
    },
}

impl TestResults {
    fn score(&self) -> Score {
        match self {
            TestResults::UnitTestResults { score, .. } => *score,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GradingSection {
    name: String, // Default: `Section <number>`
    weight: u32,  // Default: 1
    grading_mode: GradingMode,
    tests: Tests,
}

impl GradingSection {
    fn run_section_assessment(&self) -> GradingSectionResult {
        let mut result = GradingSectionResult::new(self.name.clone(), self.grading_mode);
        let test_results = self.tests.run_tests();
        result.set_test_results(test_results, self.weight);
        result
    }
}

struct GradingSectionResult {
    name: String, // Default: `Section <number>`
    score: Score,
    test_results: Option<TestResults>,
}

impl GradingSectionResult {
    fn new(name: String, grading_mode: GradingMode) -> Self {
        Self {
            name,
            score: Score::default(grading_mode),
            test_results: None,
        }
    }

    fn set_test_results(&mut self, test_results: TestResults, weight: u32) {
        self.score = test_results.score() * weight;
        self.test_results = Some(test_results);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GradingConfig {
    name: String,
    author: String,
    grading_mode: GradingMode,
    grading_sections: Vec<GradingSection>,
}

impl GradingConfig {
    fn run_assessment(&self) -> GradingResult {
        let mut result =
            GradingResult::new(self.name.clone(), self.author.clone(), self.grading_mode);

        for sec in self.grading_sections.iter() {
            result.add_section_result(sec.run_section_assessment());
        }
        result
    }
}

struct GradingResult {
    name: String,
    author: String,
    score: Score,
    grading_section_results: Vec<GradingSectionResult>,
}

impl GradingResult {
    fn new(name: String, author: String, grading_mode: GradingMode) -> Self {
        Self {
            name,
            author,
            score: Score::default(grading_mode),
            grading_section_results: vec![],
        }
    }

    fn add_section_result(&mut self, grading_section_result: GradingSectionResult) {
        self.score += grading_section_result.score;
        self.grading_section_results.push(grading_section_result);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Grader<'a> {
    config: &'a GradingConfig,
}

impl<'a> Grader<'a> {
    pub fn new(config: &'a GradingConfig) -> Self {
        Self { config }
    }
    pub fn run_assessment(&self) -> GradingResult {
        self.config.run_assessment()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
