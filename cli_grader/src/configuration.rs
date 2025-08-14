use crate::{grader::score::Mode as GradingMode, LoggingMode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
struct GradingSection {
    mode: GradingMode,
}

// TODO move this to the report module in the future.
// The configuration module should know about other modules but other modules should not
// know about it.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ReportOutput {
    Txt,
    #[default]
    Stdout,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
struct ReportSection {
    is_verbose: bool,
    output: ReportOutput,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
enum InputType {
    #[default]
    CompiledProgram,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum ProgramSpecification {
    OnlyType(InputType),
    Complete {
        /// This will be used to allow alternative argument naming for the CLI. Also, it
        /// will allow the configuration file to reference the target programs using the
        /// aliases instead of their standard name (program1, program2, program3, ... or
        /// p1, p2, p3, ...).
        ///
        /// # Example
        /// - If the alias has the value `foo`, the user may use the cli grader
        ///   in the following way: `cligrader configuration.json --p-foo program.py`
        ///   This would be an alternative to: `cligrader configuration.json program.py`
        ///
        /// - This functionality becomes more useful when we have multiple input programs,
        ///   as illustrated in this example. We will use two alias, "java" for the java program
        ///   and the "python" for the python program. The following piece of code shows the
        ///   use with the alias:
        ///   `cligrader configuration.json --program-java p1.java --p-python p2.python`
        ///   Without alias, we could have two versions, one being wrong:
        ///   `cligrader configuration.json p1.java p2.python`
        ///   `cligrader configuration.json p2.python p1.java`
        alias: String,
        input_type: InputType,
    },
}

impl Default for ProgramSpecification {
    fn default() -> Self {
        Self::OnlyType(InputType::default())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct InputSection {
    /// This vector will define all the programs that will be available in the scope of the
    /// test.
    ///
    /// # Default
    /// Defaults to only one program with the standard name: "program1" and one additional  "p1"
    input_programs: Vec<ProgramSpecification>,
}

impl Default for InputSection {
    fn default() -> Self {
        Self {
            input_programs: vec![ProgramSpecification::default()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TableHeaderType {
    Args,
    Stdout,
    Stderr,
    Status,
    Weight,
    Name,
}

// TODO (checkpoint): fix the serialized version of this enum.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum TableCellContent {
    String(String),
    I32(i32),
    U32(i32),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Table {
    row_size: usize,
    header: Vec<TableHeaderType>,
    tests: Vec<Vec<TableCellContent>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct DetailedTest {
    name: Option<String>,
    args: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    status: Option<i32>,
    weight: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct UnitTest {
    title: Option<String>,
    /// This field specify the reference to the program that will be tested by this instance
    /// of unit test. It may be the standard name of the program (`program<number>`, with
    /// `<number>` any of 1, 2, ...) or its alias. It is invalid to specify a name that was
    /// not defined in the input scope.
    program_name: String,
    table: Option<Table>,
    detailed_tests: Vec<DetailedTest>,
}

type Key = String;
type Value = String;
type Command = String;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum TestContent {
    UnitTest {
        env: Vec<(Key, Value)>,
        setup: Vec<Command>,
        teardown: Vec<Command>,
        tests: Vec<UnitTest>,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestSection {
    title: Option<String>,
    weight: Option<u32>,
    content: TestContent,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Configuration {
    title: String,
    author: Option<String>,
    #[serde(default)]
    logging_mode: LoggingMode,
    #[serde(default)]
    grading: GradingSection,
    #[serde(default)]
    report: ReportSection,
    #[serde(default)]
    input: InputSection,
    sections: Vec<TestSection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    mod test_configuration {
        use super::*;
        /// Serialize the configuration `conf`, logging its result and then deserialize it,
        /// comparing it with the original `conf`.
        macro_rules! check_json_serialization_deserialization {
            ($conf:ident) => {{
                let json = serde_json::to_string_pretty(&$conf).unwrap();
                info!("\n{json}");

                let from_json: Configuration = serde_json::from_str(json.as_str()).unwrap();

                assert!(
                    from_json == $conf,
                    "the re-deserialized version is not equal to the original one"
                );
            }};
        }
        #[test_log::test]
        fn check_serializarion_deserialization_for_configuration() {
            let c = Configuration {
                title: "configuration 1".to_string(),
                author: None,
                logging_mode: LoggingMode::Silent,
                grading: GradingSection {
                    mode: GradingMode::Weighted,
                },
                report: ReportSection {
                    is_verbose: false,
                    output: ReportOutput::Txt,
                },
                input: InputSection::default(),
                sections: vec![TestSection {
                    title: Some("Section 1".to_string()),
                    weight: Some(12),
                    content: TestContent::UnitTest {
                        env: vec![],
                        setup: vec![],
                        teardown: vec![],
                        tests: vec![UnitTest {
                            title: None,
                            program_name: "p1".to_string(),
                            table: Some(Table {
                                row_size: 3,
                                header: vec![
                                    TableHeaderType::Args,
                                    TableHeaderType::Name,
                                    TableHeaderType::Stdout,
                                ],
                                tests: vec![vec![
                                    TableCellContent::String("arg1 arg2 arg3".to_string()),
                                    TableCellContent::String("test1".to_string()),
                                    TableCellContent::String("expected".to_string()),
                                ]],
                            }),
                            detailed_tests: vec![DetailedTest {
                                name: Some("test2".to_string()),
                                args: Some("a1 a2 a3 a4".to_string()),
                                stdout: None,
                                stderr: None,
                                status: Some(23),
                                weight: Some(2),
                            }],
                        }],
                    },
                }],
            };
            check_json_serialization_deserialization!(c);
            assert!(1 == 2);
        }
    }
}
