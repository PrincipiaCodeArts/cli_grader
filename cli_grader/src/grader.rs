// TODO (checkpoint):
// - Implement tests,
// - add documentation
// - [X] organize the project in modules
// - solve lint issues
// - prepare to finish the PR

mod assessment_modalities;
mod os_interface;
mod score;

use crate::grader::assessment_modalities::unit_test::Tests;
use crate::grader::score::GradingMode;
use assessment_modalities::unit_test::TestResults;
use score::Score;

// TODO (optimization opportunity): replace `String` with &'a str for the string
// fields.

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
        use crate::grader::{
            assessment_modalities::unit_test::{assertion::Assertion, ProgramUnitAssertions},
            os_interface::TargetProgram,
        };
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
                // TODO make internals from tests transparent.
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
                            TargetProgram::new(
                                 "program1".to_string(),
                                 "cat".to_string(),
                            ),
                        )
                        .with_assertion(Assertion::new_dummy(1, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(2, false, false, false, Some(2), 3)),
                        ProgramUnitAssertions::new(
                            "assertion group 2".to_string(),
                            TargetProgram::new(
                                 "program2".to_string(),
                                 "echo".to_string(),
                            ),
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
                            TargetProgram::new("program2".to_string(), "cat2".to_string()),
                        )
                        .with_assertion(Assertion::new_dummy(3, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(4, false, false, false, Some(2), 3))
                        .with_assertion(Assertion::new_dummy(5, true, false, true, Some(2), 2)),
                        ProgramUnitAssertions::new(
                            "assertion group 4".to_string(),
                            TargetProgram::new("program4".to_string(), "echo".to_string()),
                        )
                        .with_assertion(Assertion::new_dummy(6, true, true, true, None, 2)),
                        ProgramUnitAssertions::new(
                            "assertion group 5".to_string(),
                            TargetProgram::new("program5".to_string(), "echo5".to_string()),
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
        use crate::grader::{
            assessment_modalities::unit_test::{
                assertion::Assertion, ProgramUnitAssertionResults, ProgramUnitAssertions,
            },
            os_interface::TargetProgram,
        };
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
            let target_program = TargetProgram::new("program1".to_string(), "cat".to_string());
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
                            assertion_per_program_results: vec![ProgramUnitAssertionResults::new(
                                program_unit_assertions_name,
                                target_program,
                                GradingMode::Weighted
                            )
                            .with_assertion_result(expected_assertion1)
                            .with_assertion_result(expected_assertion2)]
                        }),
                    }]
                }
            );
        }
    }
}
