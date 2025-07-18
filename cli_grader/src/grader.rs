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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug, Clone)]
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

    mod grader_tests {
        use super::*;
        use crate::grader::{
            assessment_modalities::unit_test::{
                assertion::Assertion, ProgramUnitAssertionResults, ProgramUnitAssertions,
            },
            os_interface::{CompiledProgram, Executable},
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
            let target_program = CompiledProgram::new("program1".to_string(), "cat");
            let target_program_copy = CompiledProgram::new("program1".to_string(), "cat");
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
                    Box::new(target_program_copy),
                )
                .with_assertion(assertion1)
                .with_assertion(assertion2)],
            );
            let section1 = GradingSection::new("section 1".to_string(), 1, section1_tests);

            config.add_grading_section(section1);

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
                                target_program.description(),
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
