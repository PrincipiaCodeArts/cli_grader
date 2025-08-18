use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize,
};
use std::{collections::HashSet, iter};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TableHeaderType {
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum TableCellContent {
    Int(i64),
    String(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    pub row_size: usize,
    pub header: Vec<TableHeaderType>,
    pub tests: Vec<Vec<TableCellContent>>,
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
            for (expected_type, content) in iter::zip(&header, t) {
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(try_from = "DetailedTestUnchecked")]
pub struct DetailedTest {
    pub name: Option<String>,
    pub weight: Option<u32>,
    // input
    pub args: Option<String>,
    pub stdin: Option<String>,
    // expect
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub status: Option<i32>,
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
    #[serde(default)]
    detailed_tests: Vec<DetailedTest>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(try_from = "UnitTestUnchecked")]
pub struct UnitTest {
    pub title: Option<String>,
    /// This field specify the reference to the program that will be tested by this instance
    /// of unit test. It may be the standard name of the program (`program<number>`, with
    /// `<number>` any of 1, 2, ...) or its alias. It is invalid to specify a name that was
    /// not defined in the input scope.
    ///
    /// # Caveats
    /// - If no program name is specified (None), it will logically default to the main
    ///   program (`program1` or `p1`).
    pub program_name: Option<String>,
    pub table: Option<Table>,
    pub detailed_tests: Vec<DetailedTest>,
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

        UnitTest::build(title, program_name, table, detailed_tests)
    }
}

type Key = String;
type Value = String;
type Command = String;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct UnitTestsUnchecked {
    #[serde(default)]
    env: Vec<(Key, Value)>,
    #[serde(default)]
    setup: Vec<Command>,
    #[serde(default)]
    teardown: Vec<Command>,
    tests: Vec<UnitTest>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(try_from = "UnitTestsUnchecked")]
pub struct UnitTests {
    pub env: Vec<(Key, Value)>,
    pub setup: Vec<Command>,
    pub teardown: Vec<Command>,
    pub tests: Vec<UnitTest>,
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

    #[cfg(test)]
    pub fn new_dummy() -> Self {
        Self {
            env: vec![("k1".to_string(), "v1".to_string())],
            setup: vec!["s1".to_string(), "s2".to_string()],
            teardown: vec![],
            tests: vec![UnitTest::new_dummy(1), UnitTest::new_dummy(2)],
        }
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

        UnitTests::build(env, setup, teardown, tests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod test_table_cell_content {
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
}
