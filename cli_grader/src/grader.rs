pub(crate) mod grading_tests;
pub mod score;

use crate::grader::grading_tests::{GradindTestsResult, GradingTests};
use crate::grader::score::GradingMode;
use score::Score;

/// A semantic unit that stores one type of assessment. It also has a name and a weight
/// multiplier.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GradingTestSection {
    name: String, // Default: `Section <number>`
    weight: u32,  // Default: 1
    tests: GradingTests,
}

impl GradingTestSection {
    fn run(&self, grading_mode: GradingMode) -> GradingTestSectionResult {
        let mut result = GradingTestSectionResult::new(self.name.clone(), grading_mode);
        let test_results = self.tests.run(grading_mode);
        result.set_test_results(test_results, self.weight);
        result
    }

    pub fn new(name: String, weight: u32, tests: GradingTests) -> Self {
        Self {
            name,
            weight,
            tests,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GradingTestSectionResult {
    name: String, // Default: `Section <number>`
    score: Score,
    test_results: Option<GradindTestsResult>,
}

impl GradingTestSectionResult {
    fn new(name: String, grading_mode: GradingMode) -> Self {
        Self {
            name,
            score: Score::default(grading_mode),
            test_results: None,
        }
    }

    fn set_test_results(&mut self, test_results: GradindTestsResult, weight: u32) {
        self.score = test_results.score() * weight;
        self.test_results = Some(test_results);
    }
}

/// This document has all the configuration for a complete assessment of one or more
/// executable artifacts.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GradingConfig {
    name: String,
    author: Option<String>,
    grading_mode: GradingMode,
    grading_sections: Vec<GradingTestSection>,
}

impl GradingConfig {
    pub fn new(name: String, author: Option<String>, grading_mode: GradingMode) -> Self {
        Self {
            name,
            author,
            grading_mode,
            grading_sections: vec![],
        }
    }

    pub fn add_grading_section(&mut self, grading_section: GradingTestSection) {
        self.grading_sections.push(grading_section);
    }

    fn run(&self) -> GradingResult {
        let mut result =
            GradingResult::new(self.name.clone(), self.author.clone(), self.grading_mode);

        for sec in &self.grading_sections {
            result.add_section_result(sec.run(self.grading_mode));
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GradingResult {
    name: String,
    author: Option<String>,
    score: Score,
    grading_section_results: Vec<GradingTestSectionResult>,
}

impl GradingResult {
    fn new(name: String, author: Option<String>, grading_mode: GradingMode) -> Self {
        Self {
            name,
            author,
            score: Score::default(grading_mode),
            grading_section_results: vec![],
        }
    }

    fn add_section_result(&mut self, grading_section_result: GradingTestSectionResult) {
        self.score += grading_section_result.score;
        self.grading_section_results.push(grading_section_result);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Grader<'a> {
    config: &'a GradingConfig,
}

impl<'a> Grader<'a> {
    pub fn new(config: &'a GradingConfig) -> Self {
        Self { config }
    }
    pub fn run(&self) -> GradingResult {
        self.config.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod grading_config_tests {
        use super::*;
        use crate::{
            grader::grading_tests::unit_test::{UnitTest, UnitTests, assertion::Assertion},
            input::ExecutableArtifact,
        };
        use std::vec;

        #[test]
        fn should_create_a_grading_config_with_two_grading_sections() {
            let name = "config 1";
            let author = "author 1";
            let grading_mode = GradingMode::Absolute;
            let mut config =
                GradingConfig::new(name.to_string(), Some(author.to_string()), grading_mode);

            assert_eq!(
                (
                    config.name.clone(),
                    config.author.clone(),
                    config.grading_mode
                ),
                (name.to_string(), Some(author.to_string()), grading_mode)
            );

            // Add the first grading section
            let section1_tests =
                // TODO make internals from tests transparent.
                GradingTests::UnitTests(UnitTests::new(
                    vec![("key1".to_string(), "value1".to_string())],
                    true,
                    vec![("file.txt".to_string(), "content 1".to_string())],
                    vec![("cmd1".to_string(), vec![]), ("cmd2".to_string(), vec![])],
                    vec![
                        ("tr cmd1".to_string(), vec![]),
                        ("tr cmd2".to_string(), vec![]),
                    ],
                    vec![
                        UnitTest::new(
                            "assertion group 1".to_string(),
                            ExecutableArtifact::CompiledProgram{
                                 name: "program1".to_string(),
                                 path:"cat".into(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(1, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(2, false, false, false, Some(2), 3)),
                        UnitTest::new(
                            "assertion group 2".to_string(),
                            ExecutableArtifact::CompiledProgram{
                                 name: "program2".to_string(),
                                 path:"echo".into(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(1, true, true, true, None, 2)),
                    ],
                ));
            let section1 = GradingTestSection::new("section 1".to_string(), 2, section1_tests);

            config.add_grading_section(section1.clone());

            assert_eq!(config.grading_sections, vec![section1.clone()]);

            let section2_tests =
                GradingTests::UnitTests(UnitTests::new(
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
                        UnitTest::new(
                            "assertion group 3".to_string(),
                            ExecutableArtifact::CompiledProgram {
                                name: "program2".to_string(),
                                path: "cat2".into(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(3, true, false, true, Some(2), 2))
                        .with_assertion(Assertion::new_dummy(4, false, false, false, Some(2), 3))
                        .with_assertion(Assertion::new_dummy(5, true, false, true, Some(2), 2)),
                        UnitTest::new(
                            "assertion group 4".to_string(),
                            ExecutableArtifact::CompiledProgram {
                                name: "program4".to_string(),
                                path: "echo".into(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(6, true, true, true, None, 2)),
                        UnitTest::new(
                            "assertion group 5".to_string(),
                            ExecutableArtifact::CompiledProgram {
                                name: "program5".to_string(),
                                path: "echo5".into(),
                            },
                        )
                        .with_assertion(Assertion::new_dummy(7, true, true, true, None, 2)),
                    ],
                ));
            let section2 = GradingTestSection::new("section 2".to_string(), 50, section2_tests);
            config.add_grading_section(section2.clone());

            assert_eq!(config.grading_sections, vec![section1, section2]);
        }
    }

    mod grader_tests {
        use super::*;
        use crate::{
            grader::grading_tests::unit_test::{
                UnitTest, UnitTestResult, UnitTests, UnitTestsResult, assertion::Assertion,
            },
            input::ExecutableArtifact,
        };
        use std::vec;

        #[test_log::test]
        fn should_cat_a_file() {
            let name = "Cat Project";
            let author = "author 1";
            let grading_mode = GradingMode::Weighted;
            let mut config =
                GradingConfig::new(name.to_string(), Some(author.to_string()), grading_mode);

            assert_eq!(
                (
                    config.name.clone(),
                    config.author.clone(),
                    config.grading_mode
                ),
                (name.to_string(), Some(author.to_string()), grading_mode)
            );
            let program_unit_assertions_name = "Cat from file".to_string();
            let target_program = ExecutableArtifact::CompiledProgram {
                name: "program1".to_string(),
                path: "cat".into(),
            };
            // Add the first grading section
            let assertion1 = Assertion::build(
                "should return \"hello world\"".to_string(),
                vec!["file.txt".to_string()],
                None,
                Some("hello world".to_string()),
                None,
                Some(0),
                1,
            )
            .unwrap();
            let expected_assertion1 = assertion1.expected_result(None, true, None, None, None);
            let assertion2 = Assertion::build(
                "should return \"hello   world\"".to_string(),
                vec!["file2.txt".to_string()],
                None,
                Some("hello   world".to_string()),
                None,
                Some(0),
                13,
            )
            .unwrap();
            let expected_assertion2 = assertion2.expected_result(None, true, None, None, None);
            let section1_tests = GradingTests::UnitTests(UnitTests::new(
                vec![],
                true,
                vec![
                    ("file.txt".to_string(), "hello world".to_string()),
                    ("file2.txt".to_string(), "hello   world".to_string()),
                ],
                vec![],
                vec![],
                vec![
                    UnitTest::new(program_unit_assertions_name.clone(), target_program.clone())
                        .with_assertion(assertion1)
                        .with_assertion(assertion2),
                ],
            ));
            let section1 = GradingTestSection::new("section 1".to_string(), 1, section1_tests);

            config.add_grading_section(section1.clone());

            let result = config.run();

            assert_eq!(
                result,
                GradingResult {
                    name: name.to_string(),
                    author: Some(author.to_string()),
                    score: Score::Weighted {
                        current: 14,
                        max: 14
                    },
                    grading_section_results: vec![GradingTestSectionResult {
                        name: "section 1".to_string(),
                        score: Score::Weighted {
                            current: 14,
                            max: 14
                        },
                        test_results: Some(GradindTestsResult::UnitTests(
                            UnitTestsResult::new_with(
                                Score::Weighted {
                                    current: 14,
                                    max: 14
                                },
                                vec![
                                    UnitTestResult::new(
                                        program_unit_assertions_name,
                                        target_program.name(),
                                        GradingMode::Weighted
                                    )
                                    .with_assertion_result(expected_assertion1)
                                    .with_assertion_result(expected_assertion2)
                                ]
                            )
                        )),
                    }]
                }
            );
        }
    }
}
