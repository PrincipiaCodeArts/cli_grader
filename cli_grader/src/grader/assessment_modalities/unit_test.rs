use crate::grader::score::{GradingMode, Score};
use crate::grader::{
    assessment_modalities::unit_test::assertion::Assertion, os_interface::TargetProgram,
};

pub mod assertion;

use assertion::AssertionResult;
use std::{fs, io, process};
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProgramUnitAssertions {
    name: String, // Default: `Unit testing <TargetProgram>`
    program: TargetProgram,
    assertions: Vec<Assertion>,
}
impl ProgramUnitAssertions {
    pub fn new(name: String, program: TargetProgram) -> Self {
        Self {
            name,
            program,
            assertions: vec![],
        }
    }
    pub fn with_assertion(mut self, assertion: Assertion) -> Self {
        self.assertions.push(assertion);
        self
    }

    fn add_assertion(&mut self, assertion: Assertion) {
        self.assertions.push(assertion);
    }

    fn run_assertions(
        &self,
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        files: Vec<(String, String)>,
        setup: Vec<(String, Vec<String>)>,
        teardown: Vec<(String, Vec<String>)>,
        grading_mode: GradingMode,
    ) -> io::Result<ProgramUnitAssertionResults> {
        let mut result =
            ProgramUnitAssertionResults::new(self.name.clone(), self.program.clone(), grading_mode);
        for assertion in self.assertions.iter() {
            let tmp_dir = match tempfile::tempdir() {
                Ok(dir) => dir,
                Err(err) => {
                    log::error!("error while creating a temporary directory");
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            };
            // create files
            let mut file_path;
            for (name, content) in files.iter() {
                log::debug!("Creating file: {}", name);
                file_path = tmp_dir.path().join(name);
                if let Err(err) = fs::write(&file_path, content) {
                    log::error!("error while creating the file: {}", name);
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            }
            // execute setup
            for (setup_cmd, args) in setup.iter() {
                let mut setup_cmd = process::Command::new(setup_cmd);
                setup_cmd.args(args);
                if !inherited_parent_envs {
                    setup_cmd.env_clear();
                }
                setup_cmd.current_dir(&tmp_dir);
                setup_cmd.envs(envs.clone());
                if let Err(err) = setup_cmd.output() {
                    log::error!("error while executing setup");
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            }

            // setup cmd
            let mut cmd = process::Command::new(self.program.get_path());
            if !inherited_parent_envs {
                cmd.env_clear();
            }
            cmd.current_dir(&tmp_dir);
            cmd.envs(envs.clone());
            result.add_assertion_result(assertion.unsafe_assert_cmd(cmd));

            // execute teardown
            for (teardown_cmd, args) in teardown.iter() {
                let mut teardown_cmd = process::Command::new(teardown_cmd);
                teardown_cmd.args(args);
                if !inherited_parent_envs {
                    teardown_cmd.env_clear();
                }
                teardown_cmd.current_dir(&tmp_dir);
                teardown_cmd.envs(envs.clone());
                if let Err(err) = teardown_cmd.output() {
                    log::error!("error while executing teardown");
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            }
        }
        Ok(result)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProgramUnitAssertionResults {
    name: String,
    program: TargetProgram,
    score: Score,
    assertion_results: Vec<AssertionResult>,
}

impl ProgramUnitAssertionResults {
    pub fn new(name: String, program: TargetProgram, grading_mode: GradingMode) -> Self {
        Self {
            name,
            program,
            score: Score::default(grading_mode),
            assertion_results: vec![],
        }
    }

    pub fn with_assertion_result(mut self, assertion_result: AssertionResult) -> Self {
        self.add_assertion_result(assertion_result);
        self
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

// TODO (transform to struct): instead of enum, use a trait to define the interface
// necessary to implement a Tests.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Tests {
    UnitTest {
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        files: Vec<(String, String)>,
        setup: Vec<(String, Vec<String>)>,
        teardown: Vec<(String, Vec<String>)>,
        assertions_per_program: Vec<ProgramUnitAssertions>,
    },
    // integration tests
    // performance tests
}
impl Tests {
    pub fn new_unit_test(
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        files: Vec<(String, String)>,
        setup: Vec<(String, Vec<String>)>,
        teardown: Vec<(String, Vec<String>)>,
        assertions_per_program: Vec<ProgramUnitAssertions>,
    ) -> Self {
        Self::UnitTest {
            envs,
            inherited_parent_envs,
            files,
            setup,
            teardown,
            assertions_per_program,
        }
    }
    pub fn run_tests(&self, grading_mode: GradingMode) -> TestResults {
        let mut score;
        match self {
            Tests::UnitTest {
                envs,
                files,
                inherited_parent_envs,
                setup,
                teardown,
                assertions_per_program,
            } => {
                score = Score::default(grading_mode);
                let mut test_results = vec![];
                for program_unit_assertion in assertions_per_program {
                    let res = program_unit_assertion
                        .run_assertions(
                            envs.clone(),
                            *inherited_parent_envs,
                            files.to_vec(),
                            setup.clone(),
                            teardown.clone(),
                            grading_mode,
                        )
                        // TODO (handle error): instead of panicking, it should incorporate
                        // the error into the result, making it clear why did it fail.
                        // Maybe, it would be better to incorporate the error to a more fine
                        // grained level of assertion.
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

// TODO maybe create a results module for its traits
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TestResults {
    UnitTestResults {
        score: Score,
        assertion_per_program_results: Vec<ProgramUnitAssertionResults>,
    },
}

impl TestResults {
    pub fn score(&self) -> Score {
        match self {
            TestResults::UnitTestResults { score, .. } => *score,
        }
    }
}
