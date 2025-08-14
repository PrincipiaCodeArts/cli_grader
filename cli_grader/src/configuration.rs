use crate::{LoggingMode, grader::score::Mode as GradingMode};
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
    ser::SerializeSeq,
};

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
    #[serde(rename = "exe")]
    CompiledProgram,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
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
        alias: Option<String>,
        program_type: InputType,
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum TableCellContent {
    Int(i64),
    String(String),
}

#[derive(Debug, PartialEq)]
struct Table {
    row_size: usize,
    header: Vec<TableHeaderType>,
    tests: Vec<Vec<TableCellContent>>,
}

impl Table {
    fn build(
        header: Vec<TableHeaderType>,
        tests: Vec<Vec<TableCellContent>>,
    ) -> Result<Self, &'static str> {
        if header.is_empty() {
            return Err("header must not be empty");
        }
        let row_size = header.len();
        for t in &tests {
            if t.len() != row_size {
                return Err("inconsistent test case size");
            }
        }
        // TODO check the type for each test case
        Ok(Self {
            row_size,
            header,
            tests,
        })
    }
}

impl Serialize for Table {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(1 + self.tests.len()))?;

        // serialize header
        seq.serialize_element(&self.header)?;

        for e in &self.tests {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

struct TableVisitor;

impl<'de> Visitor<'de> for TableVisitor {
    type Value = Table;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a table with a header followed by the tests")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let header: Vec<TableHeaderType> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let mut tests: Vec<Vec<TableCellContent>> =
            Vec::with_capacity(seq.size_hint().unwrap_or(1));
        let first_test: Vec<TableCellContent> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        tests.push(first_test);

        while let Some(t) = seq.next_element()? {
            tests.push(t);
        }

        match Table::build(header, tests) {
            Ok(v) => Ok(v),
            Err(msg) => Err(de::Error::custom(msg)),
        }
    }
}

impl<'de> Deserialize<'de> for Table {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(TableVisitor)
    }
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
struct UnitTests {
    env: Vec<(Key, Value)>,
    setup: Vec<Command>,
    teardown: Vec<Command>,
    tests: Vec<UnitTest>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestSection {
    title: Option<String>,
    weight: Option<u32>,
    /// # Caveats
    /// This field is optional for future purposes. In the future, it will be demanded that
    /// exactly one of `unit_tests`, `integration_tests`, or `performance_tests` is
    /// present.
    unit_tests: Option<UnitTests>,
    // integration_tests: IntegrationTests,
    // performance_tests: PerformanceTests,
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

    mod test_configuration {
        use super::*;
        /// Serialize the configuration `conf`, logging its result and then deserialize it,
        /// comparing it with the original `conf`.
        macro_rules! check_json_serialization_deserialization {
            ($conf:ident) => {{
                let json = ::serde_json::to_string_pretty(&$conf).unwrap();
                ::log::info!("\n{json}");

                let from_json: Configuration = ::serde_json::from_str(json.as_str()).unwrap();

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
                    unit_tests: Some(UnitTests {
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
                    }),
                }],
            };
            check_json_serialization_deserialization!(c);
        }

        macro_rules! check_invalid_configuration {
            ($name:ident, $conf:expr) => {
                #[test_log::test]
                #[should_panic]
                fn $name() {
                    let from_json: Configuration = ::serde_json::from_str($conf).unwrap();
                    ::log::error!("serialized:\n{}", $conf);
                    ::log::error!("deserialized:\n{from_json:#?}");
                }
            };
        }

        // TODO (checkpoint): add more tests (testing serde using unit serde_test) and check
        // each odd serde_json serialization/deserialization in isolated unit tests.
        // Also, fix any inconsistency in the invalid configuration tests, adding new ones
        // if necessary.
        check_invalid_configuration!(should_panic_with_empty_json, r#"{}"#);
        check_invalid_configuration!(
            should_panic_with_strange_data,
            r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#
        );
        check_invalid_configuration!(
            should_panic_with_invalid_grading_mode,
            r#"
        {
          "title": "configuration 1",
          "author": null,
          "logging_mode": "silent",
          "grading": {
            "mode": "invalid_mode"
          },
          "report": {
            "is_verbose": false,
            "output": "txt"
          },
          "input": {
            "input_programs": ["exe"]
          },
          "sections": [
            {
              "title": "Section 1",
              "weight": 12,
              "unit_tests": {
                "env": [],
                "setup": [],
                "teardown": [],
                "tests": [
                  {
                    "title": null,
                    "program_name": "p1",
                    "table": {
                      "row_size": 3,
                      "header": [
                        "args",
                        "name",
                        "stdout"
                      ],
                      "tests": [
                        [
                          "arg1 arg2 arg3",
                          "test1",
                          "expected"
                        ]
                      ]
                    },
                    "detailed_tests": [
                      {
                        "name": "test2",
                        "args": "a1 a2 a3 a4",
                        "stdout": null,
                        "stderr": null,
                        "status": 23,
                        "weight": 2
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }"#
        );
        check_invalid_configuration!(
            should_panic_with_invalid_report_output_mode,
            r#"
        {
          "title": "configuration 1",
          "author": null,
          "logging_mode": "silent",
          "grading": {
            "mode": "absolute"
          },
          "report": {
            "is_verbose": false,
            "output": "not_valid"
          },
          "input": {
            "input_programs": ["exe"]
          },
          "sections": [
            {
              "title": "Section 1",
              "weight": 12,
              "unit_tests": {
                "env": [],
                "setup": [],
                "teardown": [],
                "tests": [
                  {
                    "title": null,
                    "program_name": "p1",
                    "table": {
                      "row_size": 3,
                      "header": [
                        "args",
                        "name",
                        "stdout"
                      ],
                      "tests": [
                        [
                          "arg1 arg2 arg3",
                          "test1",
                          "expected"
                        ]
                      ]
                    },
                    "detailed_tests": [
                      {
                        "name": "test2",
                        "args": "a1 a2 a3 a4",
                        "stdout": null,
                        "stderr": null,
                        "status": 23,
                        "weight": 2
                      }
                    ]
                  }
                ]
              }
              
            }
          ]
        }"#
        );

        macro_rules! check_valid_configuration {
            ($name:ident, $conf:expr) => {
                #[test_log::test]
                fn $name() {
                    let _t: Configuration = ::serde_json::from_str($conf).unwrap();
                    //::log::info!("{_t:#?}");
                    //assert!(1 == 2);
                }
            };
        }

        // TODO use this to adjust the acceptable versions.
        check_valid_configuration!(
            should_accept_basic,
            r#"
        {
          "title": "Configuration ABC",
          "author": "Author ABC",
          "logging_mode": "silent",
          "grading": {
            "mode": "weighted"
          },
          "report": {
            "is_verbose": true,
            "output": "txt"
          },
          "input": {
            "input_programs": [
              "exe",
              {
                "alias": "programY",
                "program_type":"exe"
              },
              {
                "program_type":"exe"
              }
            ]
          },
          "sections": [
            {
              "title": "Section 1",
              "weight": 12,
              "unit_tests": {
                "env": [],
                "setup": [],
                "teardown": [],
                "tests": [
                  {
                    "title": null,
                    "program_name": "p1",
                    "table": [
                      ["args",                "name",  "stdout"],
                      ["arg1 arg2 arg3",      "test1", "expected1"],
                      ["a1 a2 ",              "test2", "expected2"],
                      ["arg1 arg2 arg3 arg4", "test3", "expected3"],
                      ["",                    "test4", "expected4"]
                    ],
                    "detailed_tests": [
                      {
                        "name": "test2",
                        "args": "a1 a2 a3 a4",
                        "stdout": null,
                        "stderr": null,
                        "status": 23,
                        "weight": 2
                      },
                      {
                        "name": "testABC",
                        "args": "a1",
                        "status": -1,
                        "weight": 3
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }"#
        );
    }
}
