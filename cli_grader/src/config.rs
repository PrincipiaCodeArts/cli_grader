use crate::{
    config::{
        grading_section::GradingSection, input_section::InputSection,
        report_section::ReportSection, test_section::TestSection,
    },
    input::ExecutableArtifact,
    GradingConfig, LoggingMode,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, marker, path::PathBuf};

mod grading_section;
mod input_section;
mod report_section;
mod test_section;

const DEFAULT_MAIN_PROGRAM_NAME1: &str = "program1";
const DEFAULT_MAIN_PROGRAM_NAME2: &str = "p1";
const DEFAULT_PREFIX_PROGRAM_NAME1: &str = "program";
const DEFAULT_PREFIX_PROGRAM_NAME2: &str = "p";

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

#[derive(Debug, PartialEq)]
struct NotInitialized;

#[derive(Debug, PartialEq)]
struct Initialized;

#[derive(Serialize, Debug, PartialEq)]
struct GlobalConfig<State = NotInitialized> {
    title: String,
    author: Option<String>,
    logging_mode: LoggingMode,
    grading: GradingSection,
    report: ReportSection,
    input: InputSection,
    sections: Vec<TestSection>,
    // aux
    /// In order to initialize this field, it is necessary to run `initialize` at least
    /// once.
    #[serde(skip)]
    executables_by_name: Option<HashMap<String, ExecutableArtifact>>,
    #[serde(skip)]
    _state: marker::PhantomData<State>,
}

impl<'de> Deserialize<'de> for GlobalConfig<NotInitialized> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        GlobalConfigUnchecked::deserialize(deserializer)
            .and_then(|v| GlobalConfig::try_from(v).map_err(serde::de::Error::custom))
    }
}

impl GlobalConfig<NotInitialized> {
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

        for s in &sections {
            match &s.tests {
                test_section::Tests::UnitTests(unit_tests) => {
                    for t in &unit_tests.tests {
                        if let Some(name) = &t.program_name
                            && !input.contains_program_name(name)
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
            _state: marker::PhantomData,
        })
    }

    /// It is necessary to initialize the `GlobalConfig` before using it further. It is
    /// necessary to provide additional information about the executables, which is made
    /// as the argument `program_name_to_path`, an array of tuples, mapping each program
    /// name with its path provided by the user.
    fn initialize(
        self,
        program_name_to_path: &[(&str, PathBuf)],
    ) -> Result<GlobalConfig<Initialized>, Box<(GlobalConfig<NotInitialized>, &'static str)>> {
        if self.input.input_programs_size() != program_name_to_path.len() {
            return Err(Box::new((
                self,
                "there is a different number of program names between config and user's program_name_to_path map",
            )));
        }
        let mut index_mapped = vec![false; program_name_to_path.len()];

        let mut executables_by_index = HashMap::with_capacity(program_name_to_path.len());
        for (program_name, path) in program_name_to_path {
            if !self.input.contains_program_name(program_name) {
                return Err(Box::new((self, "user program name not found in namespace")));
            }
            let program_index = self.input.get_program_index_unchecked(program_name);
            if index_mapped[program_index] {
                return Err(Box::new((self, "user program name duplicated")));
            }
            index_mapped[program_index] = true;

            let program_type = self.input.get_program_type_unchecked(program_name);

            let executable_artifact = match ExecutableArtifact::build(
                program_name.to_string(),
                path.clone(),
                program_type.into(),
            ) {
                Ok(e) => e,
                Err(err) => return Err(Box::new((self, err))),
            };
            executables_by_index.insert(program_index, executable_artifact);
        }
        let mut executables_by_name = HashMap::with_capacity(program_name_to_path.len() * 2);
        for (program_name, index) in self.input.get_program_name_by_index() {
            let executable_artifact = &executables_by_index[index];
            executables_by_name.insert(program_name.clone(), executable_artifact.clone());
        }

        Ok(GlobalConfig {
            title: self.title,
            author: self.author,
            logging_mode: self.logging_mode,
            grading: self.grading,
            report: self.report,
            input: self.input,
            sections: self.sections,
            executables_by_name: Some(executables_by_name),
            _state: marker::PhantomData,
        })
    }
}

impl GlobalConfig<Initialized> {
    fn build_grading_config(&self) -> Result<GradingConfig, &'static str> {
        let mut c = GradingConfig::new(self.title.clone(), self.author.clone(), self.grading.mode);

        let executables_by_name = self
            .executables_by_name
            .clone()
            .ok_or("executables per name map not initialized")?;

        for (i, t) in self.sections.iter().enumerate() {
            c.add_grading_section(t.build_grading_section(i + 1, &executables_by_name)?);
        }

        Ok(c)
    }
}

impl TryFrom<GlobalConfigUnchecked> for GlobalConfig<NotInitialized> {
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
                _state: marker::PhantomData::<NotInitialized>,
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
                        inherit_parent_env: false,
                        files: vec![("file 1".to_string(), "content 1".to_string())],
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

        mod test_initialize {
            use super::*;
            use crate::config::input_section::{InputType, ProgramSpecification};

            #[test]
            #[should_panic]
            fn should_panic_for_number_of_input_programs_greater_than_config() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("program3", PathBuf::from("p2")),
                    ("p1", PathBuf::from("p1")),
                    ("program2", PathBuf::from("p2")),
                    ("p4", PathBuf::from("p1")),
                ])
                .unwrap();
            }

            #[test]
            #[should_panic]
            fn should_panic_for_number_of_input_programs_less_than_config() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("p1", PathBuf::from("p1")),
                    ("program2", PathBuf::from("p2")),
                ])
                .unwrap();
            }

            #[test]
            #[should_panic]
            fn should_panic_for_invalid_input_name() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("p1", PathBuf::from("p1")),
                    ("program2", PathBuf::from("p2")),
                    ("invalid name", PathBuf::from("p2")),
                ])
                .unwrap();
            }

            #[test]
            #[should_panic]
            fn should_panic_for_duplicated_input_name() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("p1", PathBuf::from("p1")),
                    ("program2", PathBuf::from("p2")),
                    ("p2", PathBuf::from("p2 abc")),
                ])
                .unwrap();
            }
            #[test]
            #[should_panic]
            fn should_panic_with_duplicated_alias() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::Complete {
                            alias: "java".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::Complete {
                            alias: "rust".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                        ProgramSpecification::Complete {
                            alias: "python".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("p1", PathBuf::from("p1")),
                    ("java", PathBuf::from("p3")),
                    ("java", PathBuf::from("p.java")),
                    ("python", PathBuf::from("p.py")),
                    ("rust", PathBuf::from("p.rs")),
                ])
                .unwrap();
            }

            #[test]
            fn should_initialize_properly() {
                let c = GlobalConfig::build(
                    "test 1".to_string(),
                    None,
                    LoggingMode::Verbose,
                    GradingSection {
                        mode: GradingMode::Weighted,
                    },
                    ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    InputSection::build(vec![
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::Complete {
                            alias: "java".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                        ProgramSpecification::OnlyType(InputType::CompiledProgram),
                        ProgramSpecification::Complete {
                            alias: "rust".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                        ProgramSpecification::Complete {
                            alias: "python".to_string(),
                            program_type: InputType::CompiledProgram,
                        },
                    ])
                    .unwrap(),
                    vec![TestSection::new_dummy(1)],
                )
                .unwrap();
                // TODO (refactor error handling): when error handling is refactored, this
                // test will check for specific test instead of only checking for panicking.
                c.initialize(&[
                    ("p1", PathBuf::from("p1")),
                    ("program3", PathBuf::from("p3")),
                    ("java", PathBuf::from("p.java")),
                    ("python", PathBuf::from("p.py")),
                    ("rust", PathBuf::from("p.rs")),
                ])
                .unwrap();
            }
        }
        mod test_build_grader_config {
            use super::*;

            #[test]
            fn should_build_grading_config_properly() {
                let executables_by_name = HashMap::from_iter([
                    ("program1".to_string(), ExecutableArtifact::new_dummy(1)),
                    ("program2".to_string(), ExecutableArtifact::new_dummy(2)),
                    ("p1".to_string(), ExecutableArtifact::new_dummy(1)),
                    ("p2".to_string(), ExecutableArtifact::new_dummy(2)),
                ]);
                let c = GlobalConfig {
                    title: "test 1".to_string(),
                    author: None,
                    logging_mode: LoggingMode::Silent,
                    grading: GradingSection {
                        mode: GradingMode::Absolute,
                    },
                    report: ReportSection {
                        is_verbose: false,
                        output: ReportOutput::Txt,
                    },
                    input: InputSection::default(),
                    sections: vec![
                        TestSection::new_dummy(1),
                        TestSection::new_dummy(2),
                        TestSection::new_dummy(1),
                    ],
                    executables_by_name: Some(executables_by_name.clone()),
                    _state: marker::PhantomData::<Initialized>,
                };

                let mut expected =
                    GradingConfig::new("test 1".to_string(), None, GradingMode::Absolute);

                expected.add_grading_section(
                    TestSection::new_dummy(1)
                        .build_grading_section(1, &executables_by_name)
                        .unwrap(),
                );
                expected.add_grading_section(
                    TestSection::new_dummy(2)
                        .build_grading_section(2, &executables_by_name)
                        .unwrap(),
                );
                expected.add_grading_section(
                    TestSection::new_dummy(1)
                        .build_grading_section(1, &executables_by_name)
                        .unwrap(),
                );

                assert_eq!(c.build_grading_config().unwrap(), expected);
            }
        }
    }
}
