use crate::{grader::score::Mode as GradingMode, LoggingMode};
use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize,
};
use std::{collections::HashSet, iter::zip};

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
#[serde(deny_unknown_fields, default)]
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
#[serde(untagged, deny_unknown_fields)]
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
        #[serde(default)]
        program_type: InputType,
    },
}

impl Default for ProgramSpecification {
    fn default() -> Self {
        Self::OnlyType(InputType::default())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct InputSection {
    /// This vector will define all the programs that will be available in the scope of the
    /// test.
    ///
    /// # Default
    /// Defaults to only one program with the standard name: "program1" and one additional  "p1"
    #[serde(default)]
    input_programs: Vec<ProgramSpecification>,
}

impl Default for InputSection {
    fn default() -> Self {
        Self {
            input_programs: vec![ProgramSpecification::default()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum TableHeaderType {
    Name,
    Weight,
    // input
    Args,
    Stdin,
    // expect
    Stdout,
    Stderr,
    Status,
}
impl TableHeaderType {
    /// Whether the `content` is compatible with its current table column type.
    ///
    /// # Compatibility
    /// - Args, Stdout, Stderr, Name: String
    /// - Status, Weight: Int
    fn is_compatible_with(&self, content: &TableCellContent) -> bool {
        match self {
            TableHeaderType::Args
            | TableHeaderType::Stdin
            | TableHeaderType::Stdout
            | TableHeaderType::Stderr
            | TableHeaderType::Name => matches!(content, TableCellContent::String(_)),
            TableHeaderType::Status | TableHeaderType::Weight => {
                matches!(content, TableCellContent::Int(_))
            }
        }
    }

    fn is_expect_type(&self) -> bool {
        matches!(
            self,
            TableHeaderType::Stdout | TableHeaderType::Stderr | TableHeaderType::Status
        )
    }
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
        let mut has_expect_col_type = false;
        for h in &header {
            if h.is_expect_type() {
                has_expect_col_type = true;
                break;
            }
        }
        if !has_expect_col_type {
            return Err(
                "header must have at least one expect column type (stderr, stdout, or status",
            );
        }
        let header_set: HashSet<&TableHeaderType> = HashSet::from_iter(&header);
        if header_set.len() != row_size {
            return Err("header must not have duplicated elements");
        }
        for t in &tests {
            if t.len() != row_size {
                return Err("inconsistent test case size");
            }
            for (expected_type, content) in zip(&header, t) {
                if expected_type.is_compatible_with(content) {
                    continue;
                }
                return Err("inconsistent type from table test content cell");
            }
        }
        Ok(Self {
            row_size,
            header,
            tests,
        })
    }
    #[cfg(test)]
    fn new_dummy() -> Self {
        Self {
            row_size: 3,
            header: vec![
                TableHeaderType::Name,
                TableHeaderType::Args,
                TableHeaderType::Stdout,
            ],
            tests: vec![
                vec![
                    TableCellContent::String("test 1".to_string()),
                    TableCellContent::String("a1 a2 a3".to_string()),
                    TableCellContent::String("stdout 1".to_string()),
                ],
                vec![
                    TableCellContent::String("test 2".to_string()),
                    TableCellContent::String("a1 a2".to_string()),
                    TableCellContent::String("stdout 2".to_string()),
                ],
                vec![
                    TableCellContent::String("test 3".to_string()),
                    TableCellContent::String("a1 a2 a3".to_string()),
                    TableCellContent::String("stdout 3".to_string()),
                ],
            ],
        }
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
#[serde(deny_unknown_fields)]
struct DetailedTestUnchecked {
    name: Option<String>,
    weight: Option<u32>,
    // input
    args: Option<String>,
    stdin: Option<String>,
    // expect
    stdout: Option<String>,
    stderr: Option<String>,
    status: Option<i32>,
}

// Reference: https://users.rust-lang.org/t/struct-members-validation-on-serde-json-deserialize/123201/16
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "DetailedTestUnchecked")]
struct DetailedTest {
    name: Option<String>,
    weight: Option<u32>,
    // input
    args: Option<String>,
    stdin: Option<String>,
    // expect
    stdout: Option<String>,
    stderr: Option<String>,
    status: Option<i32>,
}

impl DetailedTest {
    fn build(
        name: Option<String>,
        weight: Option<u32>,
        args: Option<String>,
        stdin: Option<String>,
        stdout: Option<String>,
        stderr: Option<String>,
        status: Option<i32>,
    ) -> Result<Self, &'static str> {
        if stdout.is_none() && stderr.is_none() && status.is_none() {
            return Err("at least one of {stdout, stderr, status} must be non-null");
        }
        Ok(Self {
            name,
            weight,
            args,
            stdin,
            stdout,
            stderr,
            status,
        })
    }

    #[cfg(test)]
    fn new_dummy(n: u32) -> Self {
        Self {
            name: Some(format!("test {n}")),
            weight: Some(n),
            args: Some("arg1 arg2 arg3".to_string()),
            stdin: Some(format!("in {n}")),
            stdout: Some(format!("out {n}")),
            stderr: Some(format!("err {n}")),
            status: Some(0),
        }
    }
}

impl TryFrom<DetailedTestUnchecked> for DetailedTest {
    type Error = &'static str;

    fn try_from(value: DetailedTestUnchecked) -> Result<Self, Self::Error> {
        let DetailedTestUnchecked {
            name,
            weight,
            args,
            stdin,
            stdout,
            stderr,
            status,
        } = value;

        DetailedTest::build(name, weight, args, stdin, stdout, stderr, status)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct UnitTestUnchecked {
    title: Option<String>,
    program_name: Option<String>,
    table: Option<Table>,
    detailed_tests: Option<Vec<DetailedTest>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "UnitTestUnchecked")]
struct UnitTest {
    title: Option<String>,
    /// This field specify the reference to the program that will be tested by this instance
    /// of unit test. It may be the standard name of the program (`program<number>`, with
    /// `<number>` any of 1, 2, ...) or its alias. It is invalid to specify a name that was
    /// not defined in the input scope.
    ///
    /// # Caveats
    /// - If no program name is specified (None), it will logically default to the main
    ///   program (`program1` or `p1`).
    program_name: Option<String>,
    table: Option<Table>,
    detailed_tests: Vec<DetailedTest>,
}

impl UnitTest {
    fn build(
        title: Option<String>,
        program_name: Option<String>,
        table: Option<Table>,
        detailed_tests: Vec<DetailedTest>,
    ) -> Result<Self, &'static str> {
        if table.is_none() && detailed_tests.is_empty() {
            return Err("each UnitTest must have at least one table test or detailed test");
        }
        Ok(Self {
            title,
            program_name,
            table,
            detailed_tests,
        })
    }

    #[cfg(test)]
    fn new_dummy(n: u32) -> Self {
        Self {
            title: Some(format!("test {n}")),
            program_name: Some(format!("program {n}")),
            table: Some(Table::new_dummy()),
            detailed_tests: vec![],
        }
    }
}

impl TryFrom<UnitTestUnchecked> for UnitTest {
    type Error = &'static str;

    fn try_from(value: UnitTestUnchecked) -> Result<Self, Self::Error> {
        let UnitTestUnchecked {
            title,
            program_name,
            table,
            detailed_tests,
        } = value;

        UnitTest::build(
            title,
            program_name,
            table,
            detailed_tests.unwrap_or_default(),
        )
    }
}

type Key = String;
type Value = String;
type Command = String;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct UnitTestsUnchecked {
    env: Option<Vec<(Key, Value)>>,
    setup: Option<Vec<Command>>,
    teardown: Option<Vec<Command>>,
    tests: Vec<UnitTest>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "UnitTestsUnchecked")]
struct UnitTests {
    env: Vec<(Key, Value)>,
    setup: Vec<Command>,
    teardown: Vec<Command>,
    tests: Vec<UnitTest>,
}

impl UnitTests {
    fn build(
        env: Vec<(Key, Value)>,
        setup: Vec<Command>,
        teardown: Vec<Command>,
        tests: Vec<UnitTest>,
    ) -> Result<Self, &'static str> {
        if tests.is_empty() {
            return Err("must contain at least one test");
        }
        Ok(Self {
            env,
            setup,
            teardown,
            tests,
        })
    }
}

impl TryFrom<UnitTestsUnchecked> for UnitTests {
    type Error = &'static str;

    fn try_from(value: UnitTestsUnchecked) -> Result<Self, Self::Error> {
        let UnitTestsUnchecked {
            env,
            setup,
            teardown,
            tests,
        } = value;

        UnitTests::build(
            env.unwrap_or_default(),
            setup.unwrap_or_default(),
            teardown.unwrap_or_default(),
            tests,
        )
    }
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

    mod test_report_section {
        use crate::configuration::{ReportOutput, ReportSection};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_txt,
            ReportSection {
                is_verbose: true,
                output: ReportOutput::Txt
            },
            ReportSection
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_stdout,
            ReportSection {
                is_verbose: true,
                output: ReportOutput::Stdout
            },
            ReportSection
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_no_content_string, r#"\n"#, ReportSection);
        test_invalid_deserialization!(
            should_panic_with_wrong_is_verbose,
            r#"
        {
            "is_verbose": 123,
            "output": "txt"
        }"#,
            ReportSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_output,
            r#"
        {
            "is_verbose": true,
            "output": "txt_invalid"
        }"#,
            ReportSection
        );
        test_invalid_deserialization!(
            should_panic_with_is_verbose_as_str,
            r#"
        {
            "is_verbose": "true",
            "output": "txt"
        }"#,
            ReportSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_key,
            r#"
        {
            "is_verbose": true,
            "outputi": "txt"
        }"#,
            ReportSection
        );

        // valid deserialization
        test_valid_deserialization!(should_accept_empty_object, r#"{}"#, ReportSection);
        test_valid_deserialization!(
            should_accept_basic,
            r#"
        {
            "is_verbose": true,
            "output": "stdout"
        }"#,
            ReportSection
        );
        test_valid_deserialization!(
            should_accept_with_default_output,
            r#"
        {
            "is_verbose": true
        }"#,
            ReportSection
        );
    }

    mod test_program_specification {
        use crate::configuration::{InputType, ProgramSpecification};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_only_type,
            ProgramSpecification::OnlyType(InputType::CompiledProgram),
            ProgramSpecification
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_complete_spec,
            ProgramSpecification::Complete {
                alias: "program ABC".to_string(),
                program_type: InputType::CompiledProgram
            },
            ProgramSpecification
        );

        // invalid deserialization
        test_invalid_deserialization!(
            should_panic_with_empty_object,
            r#"{}"#,
            ProgramSpecification
        );
        test_invalid_deserialization!(should_panic_with_empty_string, r#""#, ProgramSpecification);
        test_invalid_deserialization!(
            should_panic_with_invalid_object,
            r#"{"invalid data"}"#,
            ProgramSpecification
        );
        test_invalid_deserialization!(
            should_panic_with_string,
            r#""invalid data""#,
            ProgramSpecification
        );
        test_invalid_deserialization!(
            should_panic_with_incorrect_complete_version,
            r#"
            {
                "program_type":"exee",
                "alias":null
            }"#,
            ProgramSpecification
        );
        test_invalid_deserialization!(
            should_panic_without_program_type,
            r#"
            {
                "alias":null
            }"#,
            ProgramSpecification
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_key_for_alias,
            r#"
            {
                "alia":"hello",
                "program_type":"exe"
            }"#,
            ProgramSpecification
        );
        test_invalid_deserialization!(
            should_panic_with_extra_field,
            r#"
            {
                "program_type":"exe",
                "_extra_field":123,
                "alias":"name1"
            }"#,
            ProgramSpecification
        );

        // valid deserialization
        test_valid_deserialization!(
            should_accept_basic_only_type,
            r#""exe""#,
            ProgramSpecification
        );
        test_valid_deserialization!(
            should_accept_complete_type_with_alias,
            r#"
            {
                "alias":"program ABC",
                "program_type":"exe"
            }"#,
            ProgramSpecification
        );
        test_valid_deserialization!(
            should_accept_complete_type_without_program_type,
            r#"
            {
                "alias":"program ABC"
            }"#,
            ProgramSpecification
        );
    }

    mod test_input_section {
        use crate::configuration::{InputSection, InputType, ProgramSpecification};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_empty_input_programs,
            InputSection {
                input_programs: vec![]
            },
            InputSection
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_input_programs,
            InputSection {
                input_programs: vec![
                    ProgramSpecification::OnlyType(InputType::CompiledProgram),
                    ProgramSpecification::Complete {
                        alias: "hello".to_string(),
                        program_type: InputType::CompiledProgram
                    },
                    ProgramSpecification::OnlyType(InputType::CompiledProgram),
                ]
            },
            InputSection
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_empty_string, r#""#, InputSection);
        test_invalid_deserialization!(
            should_panic_with_wrong_input_programs_type,
            r#"
        {
            "input_programs": "invalid type"
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_key,
            r#"
        {
            "input_rograms": []
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_items,
            r#"
        {
            "input_programs": [],
            "input_programs": ["exe"]
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_extra_item,
            r#"
        {
            "_comment": "abc",
            "input_programs": ["exe"]
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_input_program,
            r#"
        {
            "input_programs": ["invalid input here"]
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_input_program_format,
            r#"
        {
            "input_programs": ["exe", 123]
        }"#,
            InputSection
        );

        // valid
        test_valid_deserialization!(should_accept_empty, r#"{}"#, InputSection);
        // For serialization purpose, this case is acceptable, but for application logic, it
        // may not be accepted.
        test_valid_deserialization!(
            should_accept_empty_input_programs,
            r#"
        {
            "input_programs": []
        }"#,
            InputSection
        );
        test_valid_deserialization!(
            should_accept_with_input_programs,
            r#"
        {
            "input_programs": ["exe", {"program_type":"exe", "alias":"programB"}]
        }"#,
            InputSection
        );
    }

    mod test_table_cell_content {
        use crate::configuration::TableCellContent;

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_int,
            TableCellContent::Int(12),
            TableCellContent
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_string,
            TableCellContent::String("hello".to_string()),
            TableCellContent
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_empty_string, r#""#, TableCellContent);
        test_invalid_deserialization!(
            should_panic_with_wrong_input_programs_type,
            r#"{}"#,
            TableCellContent
        );
        test_invalid_deserialization!(should_panic_with_invalid_int, r#"12j"#, TableCellContent);
        test_invalid_deserialization!(
            should_panic_with_invalid_string,
            r#""missing_quotes"#,
            TableCellContent
        );

        // valid
        test_valid_deserialization!(should_accept_int, r#"123"#, TableCellContent);
        test_valid_deserialization!(should_accept_negative_int, r#"-123"#, TableCellContent);
        test_valid_deserialization!(
            should_accept_string,
            r#""hey this is a string""#,
            TableCellContent
        );
    }

    mod test_table {
        use crate::configuration::{Table, TableCellContent, TableHeaderType};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_table,
            Table {
                row_size: 4,
                header: vec![
                    TableHeaderType::Name,
                    TableHeaderType::Args,
                    TableHeaderType::Stdout,
                    TableHeaderType::Status
                ],
                tests: vec![
                    vec![
                        TableCellContent::String("Test 1".to_string()),
                        TableCellContent::String("a1 a2 a3".to_string()),
                        TableCellContent::String("expected 1".to_string()),
                        TableCellContent::Int(0),
                    ],
                    vec![
                        TableCellContent::String("Test 2".to_string()),
                        TableCellContent::String("a1 a3".to_string()),
                        TableCellContent::String("expected 2".to_string()),
                        TableCellContent::Int(1),
                    ],
                    vec![
                        TableCellContent::String("Test 3".to_string()),
                        TableCellContent::String("a1 a3 a4".to_string()),
                        TableCellContent::String("expected 3".to_string()),
                        TableCellContent::Int(0),
                    ]
                ]
            },
            Table
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_empty_string, r#""#, Table);
        test_invalid_deserialization!(should_panic_with_empty_map, r#"{}"#, Table);
        test_invalid_deserialization!(should_panic_with_empty_seq, r#"[]"#, Table);
        test_invalid_deserialization!(
            should_panic_with_invalid_header,
            r#"["invalid data"]"#,
            Table
        );
        test_invalid_deserialization!(should_panic_with_empty_header, r#"[[]]"#, Table);
        test_invalid_deserialization!(should_panic_with_empty_content, r#"[[], [], []]"#, Table);
        test_invalid_deserialization!(
            should_panic_with_only_header,
            r#"[["args", "status"]]"#,
            Table
        );
        test_invalid_deserialization!(
            should_panic_with_incompatible_content_size,
            r#"[["args", "status"], ["arg1 arg2", 12], ["arg2 arg1"]]"#,
            Table
        );

        test_invalid_deserialization!(
            should_panic_with_incompatible_content_type,
            r#"[
                ["args", "status"], 
                ["arg1 arg2", 12], 
                ["arg2 arg1", "12"]
            ]"#,
            Table
        );
        test_invalid_deserialization!(
            should_panic_when_header_is_not_first,
            r#"[
                ["test1", "123", "321"],
                ["test2", "1233", "3321"], 
                ["name", "stdin", "stdout"], 
                ["test3", "121", "121"] 
            ]"#,
            Table
        );
        test_invalid_deserialization!(
            should_panic_when_there_is_no_colum_of_type_expect,
            r#"[
                ["name", "args"], 
                ["test1", "arg1 arg2"],
                ["test2", "arg1 arg3"] 
            ]"#,
            Table
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_element_in_header,
            r#"[
                ["name", "stdout", "stdout"], 
                ["test1", "123", "321"],
                ["test2", "1233", "3321"],
                ["test3", "121", "121"] 
            ]"#,
            Table
        );

        // valid

        test_valid_deserialization!(
            should_accept_table_with_one_test,
            r#"[
                ["args", "status", "name"], 
                ["arg1 arg2", 12, "test1"] 
            ]"#,
            Table
        );
        test_valid_deserialization!(
            should_accept_table_with_three_test,
            r#"[
                ["name", "stdin", "stdout"], 
                ["test1", "123", "321"],
                ["test2", "1233", "3321"],
                ["test3", "121", "121"] 
            ]"#,
            Table
        );
    }

    mod test_detailed_test {
        use crate::configuration::DetailedTest;

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_full,
            DetailedTest {
                name: Some("Name 1".to_string()),
                weight: Some(2),
                args: Some("a1 a2 a3".to_string()),
                stdin: Some("input 1".to_string()),
                stdout: Some("stdout1".to_string()),
                stderr: Some("stderr1".to_string()),
                status: Some(2),
            },
            DetailedTest
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_not_full,
            DetailedTest {
                name: None,
                stdin: None,
                args: None,
                stdout: None,
                stderr: None,
                status: Some(2),
                weight: None,
            },
            DetailedTest
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_no_content_string, r#"\n"#, DetailedTest);
        test_invalid_deserialization!(should_panic_with_empty_object, r#"{}"#, DetailedTest);
        test_invalid_deserialization!(
            should_panic_with_wrong_fields,
            r#"
        {
            "wrong field":123
        }"#,
            DetailedTest
        );
        test_invalid_deserialization!(
            should_panic_without_check_fields,
            r#"
        {
            "name":"name 1",
            "args":"arg1",
            "weight":2
        }"#,
            DetailedTest
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_type,
            r#"
        {
            "name":"name 1",
            "args":"arg1",
            "status":"34",
            "weight":2
        }"#,
            DetailedTest
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_field_name,
            r#"
        {
            "name":"name 1",
            "arg":"arg1",
            "status":34,
            "weight":2
        }"#,
            DetailedTest
        );
        test_invalid_deserialization!(
            should_panic_with_extra_field,
            r#"
        {
            "name":"name 1",
            "args":"arg1",
            "_extra":23,
            "status":34,
            "weight":2
        }"#,
            DetailedTest
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_field,
            r#"
        {
            "name":"name 1",
            "args":"arg1",
            "name":"name 1",
            "status":34,
            "name":"name 1",
            "weight":2
        }"#,
            DetailedTest
        );

        // valid deserialization
        test_valid_deserialization!(
            should_accept_complete,
            r#"
        {
            "name":"name 1",
            "args":"arg1",
            "status":34,
            "stderr":"",
            "stdout":"",
            "weight":2
        }"#,
            DetailedTest
        );
        test_valid_deserialization!(
            should_accept_basic1,
            r#"
        {
            "stdin":"",
            "name":"name 1",
            "args":"arg1",
            "status":34,
            "weight":2
        }"#,
            DetailedTest
        );
        test_valid_deserialization!(
            should_accept_basic2,
            r#"
        {
            "status":34
        }"#,
            DetailedTest
        );
    }

    mod test_unit_test {
        use crate::configuration::{DetailedTest, Table, UnitTest};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_full,
            UnitTest {
                title: Some("test1".to_string()),
                program_name: Some("p1".to_string()),
                table: Some(Table::new_dummy()),
                detailed_tests: vec![DetailedTest::new_dummy(1)]
            },
            UnitTest
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_with_detailed_test,
            UnitTest {
                title: Some("test1".to_string()),
                program_name: Some("p1".to_string()),
                table: None,
                detailed_tests: vec![DetailedTest::new_dummy(1)]
            },
            UnitTest
        );

        test_serialize_and_deserialize!(
            should_serialize_deserialize_table_test,
            UnitTest {
                title: None,
                program_name: None,
                table: Some(Table::new_dummy()),
                detailed_tests: vec![]
            },
            UnitTest
        );

        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_no_content_string, r#"\n"#, UnitTest);
        test_invalid_deserialization!(should_panic_with_empty_object, r#"{}"#, UnitTest);
        test_invalid_deserialization!(
            should_panic_with_wrong_fields,
            r#"
        {
            "wrong field":123
        }"#,
            UnitTest
        );
        test_invalid_deserialization!(
            should_panic_without_tests,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "table":null
        }"#,
            UnitTest
        );
        test_invalid_deserialization!(
            should_panic_with_empty_detailed_tests,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "detailed_tests":[]
        }"#,
            UnitTest
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_field,
            r#"
        {
            "titli":"name 1",
            "program_name":"main",
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ]
        }"#,
            UnitTest
        );
        test_invalid_deserialization!(
            should_panic_with_extra_field,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "extra":false,
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ]
        }"#,
            UnitTest
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_field,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "table":[
                ["name", "args", "status"],
                ["test", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ],
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ]
        }"#,
            UnitTest
        );
        // valid deserialization
        test_valid_deserialization!(
            should_accept_with_table,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ]
        }"#,
            UnitTest
        );

        test_valid_deserialization!(
            should_accept_with_table_tests_but_empty_detailed_test,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ],
            "detailed_tests":[

            ]
        }"#,
            UnitTest
        );
        test_valid_deserialization!(
            should_accept_with_full,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "table":[
                ["name", "args", "status"],
                ["test 1", "a1 a2", 0],
                ["test 2", "a1 a3", 1],
                ["test 3", "a1 a3 a4", 2]
            ],
            "detailed_tests":[
                {
                    "args":"a1 a2 a3",
                    "name":"test 1",
                    "status":0,
                    "stdout":"hello world"
                }
            ]
        }"#,
            UnitTest
        );
        test_valid_deserialization!(
            should_accept_with_detailed_test,
            r#"
        {
            "title":"name 1",
            "program_name":"main",
            "detailed_tests":[
                {
                    "args":"a1 a2 a3",
                    "name":"test 1",
                    "status":0,
                    "stdout":"hello world"
                }
            ]
        }"#,
            UnitTest
        );
    }

    mod test_unit_tests {
        use crate::configuration::{UnitTest, UnitTests};

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_full,
            UnitTests {
                env: vec![
                    ("k1".to_string(), "v1".to_string()),
                    ("k2".to_string(), "v2".to_string())
                ],
                setup: vec!["cmd1 abc".to_string(), "cmd2 abc".to_string()],
                teardown: vec!["cmd1 abcd".to_string(), "cmd2 abcd".to_string()],
                tests: vec![UnitTest::new_dummy(0), UnitTest::new_dummy(1)]
            },
            UnitTests
        );
        test_serialize_and_deserialize!(
            should_serialize_deserialize_empty,
            UnitTests {
                env: vec![],
                setup: vec![],
                teardown: vec![],
                tests: vec![UnitTest::new_dummy(0)]
            },
            UnitTests
        );
        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_no_content_string, r#"\n"#, UnitTests);
        test_invalid_deserialization!(should_panic_with_empty_object, r#"{}"#, UnitTests);
        test_invalid_deserialization!(
            should_panic_with_wrong_field,
            r#"
        {
            "wrong field":123
        }"#,
            UnitTests
        );
        test_invalid_deserialization!(
            should_panic_without_tests,
            r#"
        {
            "setup":["cmd1 abc"],
            "tests": []
        }"#,
            UnitTests
        );
        test_invalid_deserialization!(
            should_panic_with_extra_field,
            r#"
        {
            "setup":["cmd1 abc"],
            "tests": [
                {
                    "title":"name 1",
                    "program_name":"main",
                    "detailed_tests":[
                        {
                            "args":"a1 a2 a3",
                            "name":"test 1",
                            "status":0,
                            "stdout":"hello world"
                        }
                    ]
                }
            ],
            "extra":""
        }"#,
            UnitTests
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_fields,
            r#"
        {
            "setup":["cmd1 abc"],
            "setup":23,
            "tests": [
                {
                    "title":"name 1",
                    "program_name":"main",
                    "detailed_tests":[
                        {
                            "args":"a1 a2 a3",
                            "name":"test 1",
                            "status":0,
                            "stdout":"hello world"
                        }
                    ]
                }
            ]
        }"#,
            UnitTests
        );

        // valid deserialization
        test_valid_deserialization!(
            should_accepct_full_test,
            r#"
        {
            "env":[["k1","v1"], ["k2","v2"]],
            "teardown":["cmd2 abc", "cmd3"],
            "setup":["cmd1 abc"],
            "tests": [
                {
                    "title":"name 1",
                    "program_name":"main",
                    "detailed_tests":[
                        {
                            "args":"a1 a2 a3",
                            "name":"test 1",
                            "status":0,
                            "stdout":"hello world"
                        }
                    ]
                }
            ]
        }"#,
            UnitTests
        );
    }

    mod test_configuration {
        use super::*;

        test_serialize_and_deserialize!(
            should_serialize_and_deserialize,
            Configuration {
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
            },
            Configuration
        );

        test_invalid_deserialization!(should_panic_with_empty_json, r#"{}"#, Configuration);
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
            Configuration
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
            Configuration
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
            Configuration
        );

        // TODO use this to adjust the acceptable versions.
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
            Configuration
        );
    }
}
