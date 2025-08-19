use crate::grader::grading_tests::unit_test::assertion::Assertion;
use crate::grader::score::{GradingMode, Score};

pub(crate) mod assertion;

use crate::input::ExecutableArtifact;
use assertion::AssertionResult;
use std::{fs, io, process};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnitTest {
    name: String, // Default: `Unit testing <TargetProgram>`
    // TODO does it make sense to allow multiple executables to be tested under the same set
    // of assertions?
    executable: ExecutableArtifact,
    assertions: Vec<Assertion>,
}

impl UnitTest {
    pub fn new(name: String, executable: ExecutableArtifact) -> Self {
        Self {
            name,
            executable,
            assertions: vec![],
        }
    }
    pub fn with_assertion(mut self, assertion: Assertion) -> Self {
        self.assertions.push(assertion);
        self
    }

    pub(crate) fn add_assertion(&mut self, assertion: Assertion) {
        self.assertions.push(assertion);
    }

    fn run(
        &self,
        envs: &[(String, String)],
        inherited_parent_envs: bool,
        files: &[(String, String)],
        setup: &[(String, Vec<String>)],
        teardown: &[(String, Vec<String>)],
        grading_mode: GradingMode,
    ) -> io::Result<UnitTestResult> {
        let mut result =
            UnitTestResult::new(self.name.clone(), self.executable.name(), grading_mode);
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
            for (name, content) in files {
                log::debug!("Creating file: {name}");
                file_path = tmp_dir.path().join(name);
                if let Err(err) = fs::write(&file_path, content) {
                    log::error!("error while creating the file: {name}");
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            }
            // execute setup
            let make_env_iter = || envs.iter().map(|e| (e.0.as_str(), e.1.as_str()));
            for (setup_cmd, args) in setup {
                let mut setup_cmd = process::Command::new(setup_cmd);
                setup_cmd.args(args);
                if !inherited_parent_envs {
                    setup_cmd.env_clear();
                }
                setup_cmd.current_dir(&tmp_dir);
                setup_cmd.envs(make_env_iter());
                if let Err(err) = setup_cmd.output() {
                    log::error!("error while executing setup");
                    log::debug!("error: {err:?}");
                    return Err(err);
                }
            }

            // setup cmd
            let mut cmd = self.executable.new_cmd();
            if !inherited_parent_envs {
                cmd.env_clear();
            }
            cmd.current_dir(&tmp_dir);
            cmd.envs(make_env_iter());
            result.add_assertion_result(assertion.unsafe_assert_cmd(cmd));

            // execute teardown
            for (teardown_cmd, args) in teardown {
                let mut teardown_cmd = process::Command::new(teardown_cmd);
                teardown_cmd.args(args);
                if !inherited_parent_envs {
                    teardown_cmd.env_clear();
                }
                teardown_cmd.current_dir(&tmp_dir);
                teardown_cmd.envs(make_env_iter());
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
pub struct UnitTestResult {
    name: String,
    executable_name: String,
    score: Score,
    assertion_results: Vec<AssertionResult>,
}

impl UnitTestResult {
    pub fn new(name: String, executable_name: String, grading_mode: GradingMode) -> Self {
        Self {
            name,
            executable_name,
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
            Score::Absolute(b) => {
                Score::Absolute(b && assertion_result.score() == assertion_result.max_score())
            }
            Score::Weighted { current: c, max: m } => Score::Weighted {
                current: c + assertion_result.score(),
                max: m + assertion_result.max_score(),
            },
        };
        self.assertion_results.push(assertion_result);
    }
}

// TODO add type alias to improve the signature of `UnitTest`
// type Command = String;
/// Set of `UnitTest`s.
///
/// Each `UnitTest` will be the execution of an executable artifact along with some
/// assertions. It will generate a result with the details of the process.
///
/// For each set of unit tests, it is possible to specify the environment variables, files, and
/// setup/teardown procedures to be executed just before/after each test.
///
/// # Fields
/// - `inherited_parent_envs`: whether it will inherit the environment variables from
///   parent process.
/// - files: Vec of `(<filename>, <file_content>)`.
/// - setup: Vec of (<command, Vec of args)
/// - teardown: Vec of (<command, Vec of args)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnitTests {
    envs: Vec<(String, String)>,
    inherited_parent_envs: bool,
    files: Vec<(String, String)>,
    setup: Vec<(String, Vec<String>)>,
    teardown: Vec<(String, Vec<String>)>,
    unit_tests: Vec<UnitTest>,
}

impl UnitTests {
    pub fn new(
        envs: Vec<(String, String)>,
        inherited_parent_envs: bool,
        files: Vec<(String, String)>,
        setup: Vec<(String, Vec<String>)>,
        teardown: Vec<(String, Vec<String>)>,
        assertions_per_program: Vec<UnitTest>,
    ) -> Self {
        Self {
            envs,
            inherited_parent_envs,
            files,
            setup,
            teardown,
            unit_tests: assertions_per_program,
        }
    }

    pub fn run(&self, grading_mode: GradingMode) -> UnitTestsResult {
        let mut result = UnitTestsResult::new(grading_mode);
        for program_unit_assertion in self.unit_tests.iter() {
            let res = program_unit_assertion
                .run(
                    &self.envs,
                    self.inherited_parent_envs,
                    &self.files,
                    &self.setup,
                    &self.teardown,
                    grading_mode,
                )
                // TODO (handle error): instead of panicking, it should incorporate
                // the error into the result, making it clear why did it fail.
                // Maybe, it would be better to incorporate the error to a more fine
                // grained level of assertion.
                .expect("error during assertion");
            result.add_result(res);
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnitTestsResult {
    score: Score,
    assertions_per_executable_results: Vec<UnitTestResult>,
}

impl UnitTestsResult {
    fn new(grading_mode: GradingMode) -> Self {
        Self {
            score: Score::default(grading_mode),
            assertions_per_executable_results: vec![],
        }
    }

    #[cfg(test)]
    pub fn new_with(score: Score, assertions_per_program_results: Vec<UnitTestResult>) -> Self {
        Self {
            score,
            assertions_per_executable_results: assertions_per_program_results,
        }
    }

    fn add_result(&mut self, result: UnitTestResult) {
        self.score += result.score;
        self.assertions_per_executable_results.push(result);
    }

    pub fn score(&self) -> Score {
        self.score
    }
}
