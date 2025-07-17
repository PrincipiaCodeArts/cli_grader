mod assertions;

// TODO (checkpoint):
// - Implement tests,
// - add documentation
// - organize the project in modules
// - solve lint issues
// - prepare to finish the PR

use assertions::{Assertion, AssertionResult};
use log::{debug, error};
use std::{
    fs,
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
    program: TargetProgram,
    assertions: Vec<Assertion>,
}
impl ProgramUnitAssertions {
    fn new(name: String, program: TargetProgram) -> Self {
        Self {
            name,
            program,
            assertions: vec![],
        }
    }
    fn with_assertion(mut self, assertion: Assertion) -> Self {
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
                    error!("error while creating a temporary directory");
                    debug!("error: {err:?}");
                    return Err(err);
                }
            };
            // create files
            let mut file_path;
            for (name, content) in files.iter() {
                debug!("Creating file: {}", name);
                file_path = tmp_dir.path().join(name);
                if let Err(err) = fs::write(&file_path, content) {
                    error!("error while creating the file: {}", name);
                    debug!("error: {err:?}");
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
                    error!("error while executing setup");
                    debug!("error: {err:?}");
                    return Err(err);
                }
            }

            // setup cmd
            let mut cmd = process::Command::new(&self.program.path);
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

// TODO (transform to struct): instead of enum, use a trait to define the interface
// necessary to implement a Tests.
#[derive(Debug, PartialEq, Eq, Clone)]
enum Tests {
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
    fn new_unit_test(
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
    fn run_tests(&self, grading_mode: GradingMode) -> TestResults {
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

#[derive(Debug, PartialEq, Eq, Clone)]
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
    // TODO (refactor): instead of Tests, this will be a Box<dyn Testable> to allow
    // different types of tests.
    tests: Tests,
}

impl GradingSection {
    fn run_section_assessment(&self, grading_mode: GradingMode) -> GradingSectionResult {
        let mut result = GradingSectionResult::new(self.name.clone(), grading_mode);
        let test_results = self.tests.run_tests(grading_mode);
        result.set_test_results(test_results, self.weight);
        result
    }

    fn new(name: String, weight: u32, tests: Tests) -> Self {
        Self {
            name,
            weight,
            tests,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
    fn new(name: String, author: String, grading_mode: GradingMode) -> Self {
        Self {
            name,
            author,
            grading_mode,
            grading_sections: vec![],
        }
    }

    fn add_grading_section(&mut self, grading_section: GradingSection) {
        self.grading_sections.push(grading_section);
    }

    fn run_assessment(&self) -> GradingResult {
        let mut result =
            GradingResult::new(self.name.clone(), self.author.clone(), self.grading_mode);

        for sec in self.grading_sections.iter() {
            result.add_section_result(sec.run_section_assessment(self.grading_mode));
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

    mod grading_config_tests {
        use super::*;
        use std::vec;

        #[test]
        fn should_create_a_grading_config_with_two_grading_sections() {
            let name = "config 1";
            let author = "author 1";
            let grading_mode = GradingMode::Absolute;
            let mut config = GradingConfig::new(name.to_string(), author.to_string(), grading_mode);

            assert_eq!(
                (
                    config.name.clone(),
                    config.author.clone(),
                    config.grading_mode
                ),
                (name.to_string(), author.to_string(), grading_mode)
            );

            // Add the first grading section
            let section1_tests =
                Tests::new_unit_test(
                    vec![("key1".to_string(), "value1".to_string())],
                    true,
                    vec![("file.txt".to_string(), "content 1".to_string())],
                    vec![("cmd1".to_string(), vec![]), ("cmd2".to_string(), vec![])],
                    vec![
                        ("tr cmd1".to_string(), vec![]),
                        ("tr cmd2".to_string(), vec![]),
                    ],
                    vec![
                        ProgramUnitAssertions::new(
                            "assertion group 1".to_string(),
                            TargetProgram {
                                name: "program1".to_string(),
                                path: "cat".to_string(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(1, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(2, false, false, false, Some(2), 3)),
                        ProgramUnitAssertions::new(
                            "assertion group 2".to_string(),
                            TargetProgram {
                                name: "program2".to_string(),
                                path: "echo".to_string(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(1, true, true, true, None, 2)),
                    ],
                );
            let section1 = GradingSection::new("section 1".to_string(), 2, section1_tests);

            config.add_grading_section(section1.clone());

            assert_eq!(config.grading_sections, vec![section1.clone()]);

            let section2_tests =
                Tests::new_unit_test(
                    vec![],
                    false,
                    vec![],
                    vec![
                        ("cmd1".to_string(), vec![]),
                        ("cmd2".to_string(), vec![]),
                        ("cmd3".to_string(), vec![]),
                    ],
                    vec![("tr cmd3".to_string(), vec![])],
                    vec![
                        ProgramUnitAssertions::new(
                            "assertion group 3".to_string(),
                            TargetProgram {
                                name: "program2".to_string(),
                                path: "cat2".to_string(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(3, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(4, false, false, false, Some(2), 3))
                        .with_assertion(Assertion::new_dummy(5, true, false, true, Some(2), 2)),
                        ProgramUnitAssertions::new(
                            "assertion group 4".to_string(),
                            TargetProgram {
                                name: "program4".to_string(),
                                path: "echo".to_string(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(6, true, true, true, None, 2)),
                        ProgramUnitAssertions::new(
                            "assertion group 5".to_string(),
                            TargetProgram {
                                name: "program5".to_string(),
                                path: "echo5".to_string(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(7, true, true, true, None, 2)),
                    ],
                );
            let section2 = GradingSection::new("section 2".to_string(), 50, section2_tests);
            config.add_grading_section(section2.clone());

            assert_eq!(config.grading_sections, vec![section1, section2]);
        }
    }

    mod grader_tests {
        use super::*;
        use std::vec;

        #[test_log::test]
        fn should_cat_a_file() {
            let name = "Cat Project";
            let author = "author 1";
            let grading_mode = GradingMode::Weighted;
            let mut config = GradingConfig::new(name.to_string(), author.to_string(), grading_mode);

            assert_eq!(
                (
                    config.name.clone(),
                    config.author.clone(),
                    config.grading_mode
                ),
                (name.to_string(), author.to_string(), grading_mode)
            );
            let program_unit_assertions_name = "Cat from file".to_string();
            let target_program = TargetProgram {
                name: "program1".to_string(),
                path: "cat".to_string(),
            };
            // Add the first grading section
            let assertion1 = Assertion::new(
                "should return \"hello world\"".to_string(),
                vec!["file.txt".to_string()],
                None,
                Some("hello world".to_string()),
                None,
                Some(0),
                1,
            );
            let expected_assertion1 = assertion1.expected_result(None, true, None, None, None);
            let assertion2 = Assertion::new(
                "should return \"hello   world\"".to_string(),
                vec!["file2.txt".to_string()],
                None,
                Some("hello   world".to_string()),
                None,
                Some(0),
                13,
            );
            let expected_assertion2 = assertion2.expected_result(None, true, None, None, None);
            let section1_tests = Tests::new_unit_test(
                vec![],
                true,
                vec![
                    ("file.txt".to_string(), "hello world".to_string()),
                    ("file2.txt".to_string(), "hello   world".to_string()),
                ],
                vec![],
                vec![],
                vec![ProgramUnitAssertions::new(
                    program_unit_assertions_name.clone(),
                    target_program.clone(),
                )
                .with_assertion(assertion1)
                .with_assertion(assertion2)],
            );
            let section1 = GradingSection::new("section 1".to_string(), 1, section1_tests);

            config.add_grading_section(section1.clone());

            let result = config.run_assessment();

            assert_eq!(
                result,
                GradingResult {
                    name: name.to_string(),
                    author: author.to_string(),
                    score: Score::WeightedScore {
                        current: 14,
                        max: 14
                    },
                    grading_section_results: vec![GradingSectionResult {
                        name: "section 1".to_string(),
                        score: Score::WeightedScore {
                            current: 14,
                            max: 14
                        },
                        test_results: Some(TestResults::UnitTestResults {
                            score: Score::WeightedScore {
                                current: 14,
                                max: 14
                            },
                            assertion_per_program_results: vec![ProgramUnitAssertionResults {
                                name: program_unit_assertions_name,
                                program: target_program,
                                score: Score::WeightedScore {
                                    current: 14,
                                    max: 14
                                },
                                assertion_results: vec![expected_assertion1, expected_assertion2]
                            }]
                        }),
                    }]
                }
            );
        }
    }
}
