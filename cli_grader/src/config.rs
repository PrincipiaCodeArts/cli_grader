use crate::{
    config::{
        grading_section::GradingSection, input_section::InputSection,
        report_section::ReportSection, test_section::TestSection,
    },
    input::ExecutableArtifact,
    GradingConfig, LoggingMode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod grading_section;
mod input_section;
mod report_section;
mod test_section;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct GlobalConfigUnchecked {
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "GlobalConfigUnchecked")]
struct GlobalConfig {
    title: String,
    author: Option<String>,
    logging_mode: LoggingMode,
    grading: GradingSection,
    report: ReportSection,
    input: InputSection,
    sections: Vec<TestSection>,
    // aux
    #[serde(skip)]
    executables_by_name: Option<HashMap<String, ExecutableArtifact>>,
}

impl GlobalConfig {
    fn build(
        title: String,
        author: Option<String>,
        logging_mode: LoggingMode,
        grading: GradingSection,
        report: ReportSection,
        input: InputSection,
        sections: Vec<TestSection>,
    ) -> Result<Self, &'static str> {
        if sections.is_empty() {
            return Err("at least one test section is expected");
        }

        let available_program_names = input.available_program_names().map_err(|_| "Invalid alias: duplicated name. Aliases should not have the form \"p<number>\" or \"program<number>\" nor be duplicated with other aliases")?;

        for s in &sections {
            match &s.tests {
                test_section::Tests::UnitTests(unit_tests) => {
                    for t in &unit_tests.tests {
                        if let Some(name) = &t.program_name
                            && !available_program_names.contains(name)
                        {
                            return Err("program name out of scope");
                        }
                    }
                }
            }
        }

        Ok(Self {
            title,
            author,
            logging_mode,
            grading,
            report,
            input,
            sections,
            executables_by_name: None,
        })
    }

    fn build_grader_config(&self) -> GradingConfig {
        let mut c = GradingConfig::new(self.title.clone(), self.author.clone(), self.grading.mode);

        for t in &self.sections {
            c.add_grading_section(t.build_grading_section());
        }

        c
    }

    fn set_executables_by_name(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl TryFrom<GlobalConfigUnchecked> for GlobalConfig {
    type Error = &'static str;

    fn try_from(value: GlobalConfigUnchecked) -> Result<Self, Self::Error> {
        let GlobalConfigUnchecked {
            title,
            author,
            logging_mode,
            grading,
            report,
            input,
            sections,
        } = value;

        GlobalConfig::build(
            title,
            author,
            logging_mode,
            grading,
            report,
            input,
            sections,
        )
    }
}

#[cfg(test)]
mod test_macros {
    /// From a deserialized item, test if it serializes correctly and then deserializes in
    /// sequence, maintaining the same information.
    macro_rules! test_serialize_and_deserialize {
        ($name:ident, $deserialized:expr, $type:ident, DEBUG) => {
            test_serialize_and_deserialize!($name, $deserialized, $type, true);
        };
        ($name:ident, $deserialized:expr, $type:ident $(, $fails:expr)? ) => {
            #[::test_log::test]
            fn $name() {
                let json = ::serde_json::to_string_pretty(&$deserialized).unwrap();
                ::log::info!("Serialized version:\n{json}");

                let re_deserialized: $type = ::serde_json::from_str(json.as_str()).unwrap();

                assert!(
                    re_deserialized == $deserialized,
                    "the re-deserialized version is not equal to the original one"
                );
                $(
                    if $fails {
                        assert!(false, "failed for debugging reasons");
                    }
                )?
            }
        };
    }
    macro_rules! test_invalid_deserialization {
        ($name:ident, $serialized:expr, $type:ident) => {
            #[test_log::test]
            #[should_panic]
            fn $name() {
                let from_json: $type = ::serde_json::from_str($serialized).unwrap();
                ::log::error!("serialized:\n{}", $serialized);
                ::log::error!("deserialized:\n{from_json:#?}");
            }
        };
    }

    macro_rules! test_valid_deserialization {
        ($name:ident, $serialized:expr, $type:ident) => {
            #[test_log::test]
            fn $name() {
                let _t: $type = ::serde_json::from_str($serialized).unwrap();
                //::log::info!("{_t:#?}");
                //assert!(1 == 2);
            }
        };
    }

    // export the macros
    pub(crate) use test_invalid_deserialization;
    pub(crate) use test_serialize_and_deserialize;
    pub(crate) use test_valid_deserialization;
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_configuration {
        use super::*;
        use crate::config::test_section::Tests;
        use crate::grader::score::GradingMode;
        use crate::{
            config::{
                test_macros::{
                    test_invalid_deserialization, test_serialize_and_deserialize,
                    test_valid_deserialization,
                },
                test_section::unit_tests::{
                    DetailedTest, Table, TableCellContent, TableHeaderType, UnitTest, UnitTests,
                },
            },
            report::ReportOutput,
        };

        test_serialize_and_deserialize!(
            should_serialize_and_deserialize,
            GlobalConfig {
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
                    tests: Tests::UnitTests(UnitTests {
                        env: vec![],
                        setup: vec![],
                        teardown: vec![],
                        tests: vec![UnitTest {
                            title: None,
                            program_name: Some("p1".to_string()),
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
                                stdin: None,
                                stdout: None,
                                stderr: None,
                                status: Some(23),
                                weight: Some(2),
                            }],
                        }],
                    }),
                }],
                executables_by_name: None,
            },
            GlobalConfig
        );

        // invalid
        test_invalid_deserialization!(should_panic_with_empty_json, r#"{}"#, GlobalConfig);
        test_invalid_deserialization!(
            should_panic_with_strange_data,
            r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#,
            GlobalConfig
        );
        test_invalid_deserialization!(
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
        }"#,
            GlobalConfig
        );
        test_invalid_deserialization!(
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
        }"#,
            GlobalConfig
        );
        test_invalid_deserialization!(
            should_panic_with_empty_section,
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
                "alias": "programZ",
                "program_type":"exe"
              }
            ]
          },
          "sections": []
        }"#,
            GlobalConfig
        );
        test_invalid_deserialization!(
            should_panic_with_program_name_out_of_scope,
            r#"
        {
          "title": "Configuration ABC",
          "author": "Author ABC",
          "input": {
            "input_programs": [
              "exe",
              {
                "alias": "programY",
                "program_type":"exe"
              },
              {
                "alias": "programZ",
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
                    "program_name": "p4",
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
            },
            {
              "title": "Section 2",
              "weight": 2,
              "unit_tests": {
                "env": [],
                "setup": [],
                "teardown": [],
                "tests": [
                  {
                    "title": null,
                    "program_name": "programY",
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
                  },
                  {
                    "program_name": "program3",
                    "table": [
                      ["args",                "name",  "stdout"],
                      ["arg1 arg2 arg3",      "test1", "expected1"],
                      ["a1 a2 ",              "test2", "expected2"],
                      ["arg1 arg2 arg3 arg4", "test3", "expected3"],
                      ["",                    "test4", "expected4"]
                    ]
                  },
                  {
                    "program_name": "programZ",
                    "table": [
                      ["args",                "name",  "stdout"],
                      ["arg1 arg2 arg3",      "test1", "expected1"],
                      ["a1 a2 ",              "test2", "expected2"],
                      ["arg1 arg2 arg3 arg4", "test3", "expected3"],
                      ["",                    "test4", "expected4"]
                    ]
                  }
                ]
              }
            }
          ]
        }"#,
            GlobalConfig
        );

        // valid
        test_valid_deserialization!(
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
                "alias": "programZ",
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
        }"#,
            GlobalConfig
        );
        test_valid_deserialization!(
            should_accept_programs_with_aliases,
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
                "alias": "programZ",
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
            },
            {
              "title": "Section 2",
              "weight": 2,
              "unit_tests": {
                "env": [],
                "setup": [],
                "teardown": [],
                "tests": [
                  {
                    "title": null,
                    "program_name": "programY",
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
                  },
                  {
                    "program_name": "program3",
                    "table": [
                      ["args",                "name",  "stdout"],
                      ["arg1 arg2 arg3",      "test1", "expected1"],
                      ["a1 a2 ",              "test2", "expected2"],
                      ["arg1 arg2 arg3 arg4", "test3", "expected3"],
                      ["",                    "test4", "expected4"]
                    ]
                  },
                  {
                    "program_name": "programZ",
                    "table": [
                      ["args",                "name",  "stdout"],
                      ["arg1 arg2 arg3",      "test1", "expected1"],
                      ["a1 a2 ",              "test2", "expected2"],
                      ["arg1 arg2 arg3 arg4", "test3", "expected3"],
                      ["",                    "test4", "expected4"]
                    ]
                  }
                ]
              }
            }
          ]
        }"#,
            GlobalConfig
        );
        test_valid_deserialization!(
            should_accept_with_mandatory_fields_only,
            r#"
        {
          "title": "Configuration ABC",
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
        }"#,
            GlobalConfig
        );
    }
}
