use crate::{
    grader::grading_tests::unit_test::{
        UnitTest as GradingUnitTest, UnitTests as GradingUnitTests,
        assertion::Assertion as UnitTestAssertion,
    },
    input::ExecutableArtifact,
};
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
    ser::SerializeSeq,
};
use shlex::Shlex;
use std::{
    collections::{HashMap, HashSet},
    iter, panic,
};

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
impl TableCellContent {
    fn extract_string(&self) -> String {
        match self {
            TableCellContent::String(s) => s.clone(),
            _ => panic!("expected string"),
        }
    }
    fn extract_u32(&self) -> u32 {
        match self {
            TableCellContent::Int(i) => *i as u32,
            _ => panic!("expected u32"),
        }
    }
    fn extract_i32(&self) -> i32 {
        match self {
            TableCellContent::Int(i) => *i as i32,
            _ => panic!("expected i32"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    // TODO (remove_field): check if row_size is really necessary, and remove it, if not.
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
    fn build_grading_assertions(
        &self,
        mut n: usize,
    ) -> Result<Vec<UnitTestAssertion>, &'static str> {
        let mut assertions = vec![];
        for t in &self.tests {
            let mut name = format!("Assertion {n}");
            n += 1;
            let mut args = vec![];
            let mut stdin: Option<String> = None;
            let mut stdout: Option<String> = None;
            let mut stderr: Option<String> = None;
            let mut status: Option<i32> = None;
            let mut weight: u32 = 1;
            for (i, h) in self.header.iter().enumerate() {
                match h {
                    TableHeaderType::Name => name = t[i].extract_string(),
                    TableHeaderType::Weight => weight = t[i].extract_u32(),
                    TableHeaderType::Args => {
                        let args_string = t[i].extract_string();
                        let mut lex = Shlex::new(args_string.as_str());
                        for arg in lex.by_ref() {
                            args.push(arg);
                        }
                        if lex.had_error {
                            return Err("invalid args string");
                        }
                    }
                    TableHeaderType::Stdin => stdin = Some(t[i].extract_string()),
                    TableHeaderType::Stdout => stdout = Some(t[i].extract_string()),
                    TableHeaderType::Stderr => stderr = Some(t[i].extract_string()),
                    TableHeaderType::Status => status = Some(t[i].extract_i32()),
                }
            }
            if let Ok(assertion) =
                UnitTestAssertion::build(name, args, stdin, stdout, stderr, status, weight)
            {
                assertions.push(assertion);
                continue;
            }
            return Err("could not build assertion properly");
        }
        Ok(assertions)
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

    fn build_grading_assertion(&self, n: usize) -> Result<UnitTestAssertion, &'static str> {
        let DetailedTest {
            name,
            weight,
            args: args_string,
            stdin,
            stdout,
            stderr,
            status,
        } = self;
        let mut args = vec![];

        if let Some(args_string) = args_string {
            let mut lex = Shlex::new(args_string.as_str());
            for arg in lex.by_ref() {
                args.push(arg);
            }
            if lex.had_error {
                return Err("invalid args string");
            }
        }
        UnitTestAssertion::build(
            name.clone().unwrap_or(format!("Assertion {n}")),
            args,
            stdin.clone(),
            stdout.clone(),
            stderr.clone(),
            *status,
            weight.unwrap_or(1),
        )
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

    fn build_grading_unit_test(
        &self,
        n: usize,
        executables_by_name: &HashMap<String, ExecutableArtifact>,
    ) -> Result<GradingUnitTest, &'static str> {
        // try to get the executable
        let default_name = format!("Unit Test {n}");
        // TODO (move to a const)
        let default_program_name = "program1".to_string();
        let executable =
            executables_by_name.get(self.program_name.as_ref().unwrap_or(&default_program_name));
        if executable.is_none() {
            return Err("executable not found");
        }
        let mut unit_test = GradingUnitTest::new(
            self.title.as_ref().unwrap_or(&default_name).clone(),
            executable.unwrap().clone(),
        );

        // add assertions
        // table
        if let Some(table) = &self.table {
            unit_test.add_assertions(table.build_grading_assertions(1)?);
        }

        // detailed tests
        let mut n = unit_test.size() + 1;
        for d in &self.detailed_tests {
            unit_test.add_assertion(d.build_grading_assertion(n)?);
            n += 1;
        }
        Ok(unit_test)
    }

    #[cfg(test)]
    fn new_dummy(n: u32) -> Self {
        Self {
            title: Some(format!("test {n}")),
            program_name: Some(format!("program{n}")),
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
type FileContent = String;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct UnitTestsUnchecked {
    #[serde(default)]
    env: Vec<(Key, Value)>,
    #[serde(default)]
    inherit_parent_env: Option<bool>,
    #[serde(default)]
    files: Vec<(String, FileContent)>,
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
    pub inherit_parent_env: bool,
    pub files: Vec<(String, FileContent)>,
    pub setup: Vec<Command>,
    pub teardown: Vec<Command>,
    pub tests: Vec<UnitTest>,
}

impl UnitTests {
    fn build(
        env: Vec<(Key, Value)>,
        inherit_parent_env: bool,
        files: Vec<(String, FileContent)>,
        setup: Vec<Command>,
        teardown: Vec<Command>,
        tests: Vec<UnitTest>,
    ) -> Result<Self, &'static str> {
        if tests.is_empty() {
            return Err("must contain at least one test");
        }
        Ok(Self {
            env,
            inherit_parent_env,
            files,
            setup,
            teardown,
            tests,
        })
    }

    fn build_grading_unit_tests(
        &self,
        executables_by_name: &HashMap<String, ExecutableArtifact>,
    ) -> Result<GradingUnitTests, &'static str> {
        let mut unit_tests = vec![];

        fn process_raw_string_commands(
            commands: &[String],
        ) -> Result<Vec<(String, Vec<String>)>, &'static str> {
            let mut processed_commands = vec![];
            for command in commands {
                let mut lex = Shlex::new(command.as_str());
                let command_name = match lex.next() {
                    Some(c) => c,
                    None => return Err("missing command"),
                };
                let mut processed_command = (command_name, vec![]);
                for arg in lex.by_ref() {
                    processed_command.1.push(arg);
                }
                if lex.had_error {
                    return Err("invalid args string");
                }
                processed_commands.push(processed_command);
            }
            Ok(processed_commands)
        }

        // add unit tests
        for (i, t) in self.tests.iter().enumerate() {
            unit_tests.push(t.build_grading_unit_test(i + 1, executables_by_name)?);
        }
        Ok(GradingUnitTests::new(
            self.env.clone(),
            self.inherit_parent_env,
            self.files.clone(),
            process_raw_string_commands(&self.setup)?,
            process_raw_string_commands(&self.teardown)?,
            unit_tests,
        ))
    }

    #[cfg(test)]
    pub fn new_dummy() -> Self {
        Self {
            env: vec![("k1".to_string(), "v1".to_string())],
            inherit_parent_env: true,
            files: vec![("file1.txt".to_string(), "hello\nworld".to_string())],
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
            inherit_parent_env,
            files,
            setup,
            teardown,
            tests,
        } = value;

        UnitTests::build(
            env,
            inherit_parent_env.unwrap_or(true),
            files,
            setup,
            teardown,
            tests,
        )
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

        mod test_build_grading_assertions {
            use super::*;

            #[test]
            #[should_panic]
            fn should_panic_when_assertion_is_not_built_properly() {
                let invalid_table = Table {
                    row_size: 3,
                    header: vec![
                        TableHeaderType::Name,
                        TableHeaderType::Stdin,
                        TableHeaderType::Weight,
                    ],
                    tests: vec![
                        vec![
                            TableCellContent::String("name 1".to_string()),
                            TableCellContent::String("stdin 1".to_string()),
                            TableCellContent::Int(1),
                        ],
                        vec![
                            TableCellContent::String("name 2".to_string()),
                            TableCellContent::String("stdin 2".to_string()),
                            TableCellContent::Int(2),
                        ],
                    ],
                };
                invalid_table.build_grading_assertions(1).unwrap();
            }
            #[test]
            fn should_match_a_simple_table_test() {
                let t = Table::build(
                    vec![TableHeaderType::Name, TableHeaderType::Status],
                    vec![vec![
                        TableCellContent::String("test 1".to_string()),
                        TableCellContent::Int(0),
                    ]],
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertions(1).unwrap(),
                    vec![UnitTestAssertion::new(
                        "test 1".to_string(),
                        vec![],
                        None,
                        None,
                        None,
                        Some(0),
                        1,
                    )]
                );
            }
            #[test]
            fn should_match_args_correctly() {
                let t = Table::build(
                    vec![TableHeaderType::Args, TableHeaderType::Status],
                    vec![
                        vec![
                            TableCellContent::String("arg1".to_string()),
                            TableCellContent::Int(0),
                        ],
                        vec![
                            TableCellContent::String("arg1    ".to_string()),
                            TableCellContent::Int(0),
                        ],
                        vec![
                            TableCellContent::String("\"arg1    \"".to_string()),
                            TableCellContent::Int(0),
                        ],
                        vec![
                            TableCellContent::String(
                                "arg1 arg2\t\targ3  \"this is an arg\"    arg 5".to_string(),
                            ),
                            TableCellContent::Int(0),
                        ],
                    ],
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertions(1).unwrap(),
                    vec![
                        UnitTestAssertion::new(
                            "Assertion 1".to_string(),
                            vec!["arg1".to_string()],
                            None,
                            None,
                            None,
                            Some(0),
                            1,
                        ),
                        UnitTestAssertion::new(
                            "Assertion 2".to_string(),
                            vec!["arg1".to_string()],
                            None,
                            None,
                            None,
                            Some(0),
                            1,
                        ),
                        UnitTestAssertion::new(
                            "Assertion 3".to_string(),
                            vec!["arg1    ".to_string()],
                            None,
                            None,
                            None,
                            Some(0),
                            1,
                        ),
                        UnitTestAssertion::new(
                            "Assertion 4".to_string(),
                            vec![
                                "arg1".to_string(),
                                "arg2".to_string(),
                                "arg3".to_string(),
                                "this is an arg".to_string(),
                                "arg".to_string(),
                                "5".to_string(),
                            ],
                            None,
                            None,
                            None,
                            Some(0),
                            1,
                        )
                    ]
                );
            }

            #[test]
            fn should_match_a_complex_table_test() {
                let t = Table::build(
                    vec![
                        TableHeaderType::Name,
                        TableHeaderType::Status,
                        TableHeaderType::Weight,
                        TableHeaderType::Stdout,
                    ],
                    vec![
                        vec![
                            TableCellContent::String("test 1".to_string()),
                            TableCellContent::Int(0),
                            TableCellContent::Int(1),
                            TableCellContent::String("stdout 1".to_string()),
                        ],
                        vec![
                            TableCellContent::String("test 2".to_string()),
                            TableCellContent::Int(0),
                            TableCellContent::Int(2),
                            TableCellContent::String("stdout 2".to_string()),
                        ],
                        vec![
                            TableCellContent::String("test 3".to_string()),
                            TableCellContent::Int(1),
                            TableCellContent::Int(3),
                            TableCellContent::String("".to_string()),
                        ],
                    ],
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertions(1).unwrap(),
                    vec![
                        UnitTestAssertion::new(
                            "test 1".to_string(),
                            vec![],
                            None,
                            Some("stdout 1".to_string()),
                            None,
                            Some(0),
                            1,
                        ),
                        UnitTestAssertion::new(
                            "test 2".to_string(),
                            vec![],
                            None,
                            Some("stdout 2".to_string()),
                            None,
                            Some(0),
                            2,
                        ),
                        UnitTestAssertion::new(
                            "test 3".to_string(),
                            vec![],
                            None,
                            Some("".to_string()),
                            None,
                            Some(1),
                            3,
                        ),
                    ]
                );
            }
            #[test]
            fn should_match_a_with_default_table_test_name() {
                let t = Table::build(
                    vec![
                        TableHeaderType::Status,
                        TableHeaderType::Weight,
                        TableHeaderType::Stdout,
                    ],
                    vec![
                        vec![
                            TableCellContent::Int(0),
                            TableCellContent::Int(1),
                            TableCellContent::String("stdout 1".to_string()),
                        ],
                        vec![
                            TableCellContent::Int(0),
                            TableCellContent::Int(2),
                            TableCellContent::String("stdout 2".to_string()),
                        ],
                        vec![
                            TableCellContent::Int(1),
                            TableCellContent::Int(3),
                            TableCellContent::String("".to_string()),
                        ],
                    ],
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertions(2).unwrap(),
                    vec![
                        UnitTestAssertion::new(
                            "Assertion 2".to_string(),
                            vec![],
                            None,
                            Some("stdout 1".to_string()),
                            None,
                            Some(0),
                            1,
                        ),
                        UnitTestAssertion::new(
                            "Assertion 3".to_string(),
                            vec![],
                            None,
                            Some("stdout 2".to_string()),
                            None,
                            Some(0),
                            2,
                        ),
                        UnitTestAssertion::new(
                            "Assertion 4".to_string(),
                            vec![],
                            None,
                            Some("".to_string()),
                            None,
                            Some(1),
                            3,
                        ),
                    ]
                );
            }
        }
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

        mod test_build_grading_assertion {
            use super::*;

            #[test]
            #[should_panic]
            fn should_panic_when_assertion_is_not_built_properly() {
                let invalid_table = DetailedTest {
                    name: None,
                    weight: Some(1),
                    args: None,
                    stdin: Some("stdin 1".to_string()),
                    stdout: None,
                    stderr: None,
                    status: None,
                };
                invalid_table.build_grading_assertion(1).unwrap();
            }
            #[test]
            fn should_match_a_simple_detailed_test() {
                let t = DetailedTest::build(
                    None,
                    None,
                    Some("arg1 arg2 \" an arg \"".to_string()),
                    Some("".to_string()),
                    Some("".to_string()),
                    None,
                    None,
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertion(10).unwrap(),
                    UnitTestAssertion::build(
                        "Assertion 10".to_string(),
                        vec![
                            "arg1".to_string(),
                            "arg2".to_string(),
                            " an arg ".to_string()
                        ],
                        Some("".to_string()),
                        Some("".to_string()),
                        None,
                        None,
                        1,
                    )
                    .unwrap()
                );
            }

            #[test]
            fn should_match_a_full_detailed_test() {
                let t = DetailedTest::build(
                    Some("name abc".to_string()),
                    Some(12),
                    Some("a1 a2 a3".to_string()),
                    Some("stdin abc".to_string()),
                    Some("stdout abc".to_string()),
                    Some("stderr abc".to_string()),
                    Some(0),
                )
                .unwrap();
                assert_eq!(
                    t.build_grading_assertion(10).unwrap(),
                    UnitTestAssertion::build(
                        "name abc".to_string(),
                        vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
                        Some("stdin abc".to_string()),
                        Some("stdout abc".to_string()),
                        Some("stderr abc".to_string()),
                        Some(0),
                        12,
                    )
                    .unwrap()
                );
            }
        }
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

        mod test_build_grading_unit_test {
            use super::*;
            use std::path::PathBuf;
            #[test]
            #[should_panic]
            fn should_panic_when_there_is_no_executable_for_given_program() {
                let u = UnitTest::build(
                    Some("UnitTest1".to_string()),
                    Some("some program".to_string()),
                    Some(
                        Table::build(
                            vec![
                                TableHeaderType::Name,
                                TableHeaderType::Args,
                                TableHeaderType::Status,
                            ],
                            vec![
                                vec![
                                    TableCellContent::String("test 1".to_string()),
                                    TableCellContent::String("a1 a2".to_string()),
                                    TableCellContent::Int(0),
                                ],
                                vec![
                                    TableCellContent::String("test 2".to_string()),
                                    TableCellContent::String("a1 a2 a3".to_string()),
                                    TableCellContent::Int(2),
                                ],
                            ],
                        )
                        .unwrap(),
                    ),
                    vec![],
                )
                .unwrap();

                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                let executables_by_name = HashMap::from_iter([
                    ("not some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                    ("p1".to_string(), executable.clone()),
                ]);
                u.build_grading_unit_test(2, &executables_by_name).unwrap();
            }

            #[test]
            #[should_panic]
            fn should_panic_when_table_is_invalid() {
                let invalid_unit_test = UnitTest {
                    title: Some("UnitTest1".to_string()),
                    program_name: Some("some program".to_string()),
                    table: Some(Table {
                        row_size: 2,
                        header: vec![TableHeaderType::Name, TableHeaderType::Args],
                        tests: vec![
                            vec![
                                TableCellContent::String("test 1".to_string()),
                                TableCellContent::String("a1 a2".to_string()),
                                TableCellContent::Int(0),
                            ],
                            vec![
                                TableCellContent::String("test 2".to_string()),
                                TableCellContent::String("a1 a2 a3".to_string()),
                                TableCellContent::Int(2),
                            ],
                        ],
                    }),
                    detailed_tests: vec![],
                };

                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                let executables_by_name = HashMap::from_iter([
                    ("not some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                    ("p1".to_string(), executable.clone()),
                ]);
                invalid_unit_test
                    .build_grading_unit_test(2, &executables_by_name)
                    .unwrap();
            }

            #[test]
            fn should_accept_unit_test_with_table_tests_and_detailed_tests() {
                let u = UnitTest::build(
                    Some("UnitTest1".to_string()),
                    Some("some program".to_string()),
                    Some(
                        Table::build(
                            vec![
                                TableHeaderType::Name,
                                TableHeaderType::Args,
                                TableHeaderType::Status,
                            ],
                            vec![
                                vec![
                                    TableCellContent::String("test 1".to_string()),
                                    TableCellContent::String("a1 a2".to_string()),
                                    TableCellContent::Int(0),
                                ],
                                vec![
                                    TableCellContent::String("test 2".to_string()),
                                    TableCellContent::String("a1 a2 a3".to_string()),
                                    TableCellContent::Int(2),
                                ],
                            ],
                        )
                        .unwrap(),
                    ),
                    vec![
                        DetailedTest::build(
                            None,
                            Some(2),
                            Some("a b c".to_string()),
                            Some("stdin".to_string()),
                            Some("".to_string()),
                            None,
                            Some(3),
                        )
                        .unwrap(),
                        DetailedTest::build(
                            Some("test abc".to_string()),
                            None,
                            Some("a b".to_string()),
                            Some("stdin".to_string()),
                            Some("".to_string()),
                            None,
                            Some(3),
                        )
                        .unwrap(),
                    ],
                )
                .unwrap();

                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                // TODO (optimization): make the executables by name a map from string to a
                // reference to an executable instead of the executable itself.
                let executables_by_name = HashMap::from_iter([
                    ("some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                ]);

                assert_eq!(
                    u.build_grading_unit_test(2, &executables_by_name).unwrap(),
                    GradingUnitTest::new_for_test(
                        "UnitTest1".to_string(),
                        executable,
                        vec![
                            UnitTestAssertion::build(
                                "test 1".to_string(),
                                vec!["a1".to_string(), "a2".to_string()],
                                None,
                                None,
                                None,
                                Some(0),
                                1
                            )
                            .unwrap(),
                            UnitTestAssertion::build(
                                "test 2".to_string(),
                                vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
                                None,
                                None,
                                None,
                                Some(2),
                                1
                            )
                            .unwrap(),
                            UnitTestAssertion::build(
                                "Assertion 3".to_string(),
                                vec!["a".to_string(), "b".to_string(), "c".to_string()],
                                Some("stdin".to_string()),
                                Some("".to_string()),
                                None,
                                Some(3),
                                2
                            )
                            .unwrap(),
                            UnitTestAssertion::build(
                                "test abc".to_string(),
                                vec!["a".to_string(), "b".to_string()],
                                Some("stdin".to_string()),
                                Some("".to_string()),
                                None,
                                Some(3),
                                1
                            )
                            .unwrap()
                        ]
                    )
                );
            }
        }
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
                inherit_parent_env: true,
                files: vec![("file 1".to_string(), "content 1".to_string())],

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
                inherit_parent_env: true,
                files: vec![],
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
            should_accept_minimal_test,
            r#"
        {
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
        test_valid_deserialization!(
            should_accept_full_test,
            r#"
        {
            "env":[["k1","v1"], ["k2","v2"]],
            "teardown":["cmd2 abc", "cmd3"],
            "setup":["cmd1 abc"],
            "inherit_parent_env":false,
            "files":[
                ["file1.txt", "hello\nworld\n\n"],
                ["file2.txt", "hello\nworld2\n\n"]
            ],
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

        mod test_build_grading_unit_tests {
            use super::*;
            use std::path::PathBuf;
            #[test]
            #[should_panic]
            fn should_panic_when_there_is_invalid_command_in_setup() {
                let r = UnitTests::build(
                    vec![],
                    true,
                    vec![],
                    vec![
                        "valid command1".to_string(),
                        "".to_string(),
                        "command1 a b c".to_string(),
                    ],
                    vec![],
                    vec![UnitTest::new_dummy(1), UnitTest::new_dummy(2)],
                )
                .unwrap();

                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                let executables_by_name = HashMap::from_iter([
                    ("some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                    ("program2".to_string(), executable.clone()),
                    ("p1".to_string(), executable.clone()),
                ]);
                r.build_grading_unit_tests(&executables_by_name).unwrap();
            }

            #[test]
            #[should_panic]
            fn should_panic_when_there_is_invalid_command_in_teardown() {
                let r = UnitTests::build(
                    vec![],
                    false,
                    vec![],
                    vec![],
                    vec![
                        "valid command1".to_string(),
                        "".to_string(),
                        "command1 a b c".to_string(),
                    ],
                    vec![UnitTest::new_dummy(1), UnitTest::new_dummy(2)],
                )
                .unwrap();

                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                let executables_by_name = HashMap::from_iter([
                    ("some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                    ("program2".to_string(), executable.clone()),
                    ("p1".to_string(), executable.clone()),
                ]);
                r.build_grading_unit_tests(&executables_by_name).unwrap();
            }

            #[test]
            fn should_correctly_build_unit_tests() {
                let env = vec![
                    ("k1".to_string(), "v1".to_string()),
                    ("k2".to_string(), "v2".to_string()),
                ];
                let files = vec![("f1".to_string(), "v1".to_string())];
                let u = UnitTests::build(
                    env.clone(),
                    false,
                    files.clone(),
                    vec![
                        "command1 a b c \"hey there\"".to_string(),
                        "command2 a b c".to_string(),
                    ],
                    vec!["cm1 a b c".to_string(), "cm2 a b c".to_string()],
                    vec![
                        UnitTest::new_dummy(1),
                        UnitTest::new_dummy(2),
                        UnitTest::new_dummy(1),
                    ],
                )
                .unwrap();
                let executable = ExecutableArtifact::CompiledProgram {
                    name: "some name".to_string(),
                    path: PathBuf::new(),
                };
                let executables_by_name = HashMap::from_iter([
                    ("some program".to_string(), executable.clone()),
                    ("program1".to_string(), executable.clone()),
                    ("program2".to_string(), executable.clone()),
                    ("p1".to_string(), executable.clone()),
                ]);
                assert_eq!(
                    u.build_grading_unit_tests(&executables_by_name).unwrap(),
                    GradingUnitTests::new(
                        env,
                        false,
                        files,
                        vec![
                            (
                                "command1".to_string(),
                                vec![
                                    "a".to_string(),
                                    "b".to_string(),
                                    "c".to_string(),
                                    "hey there".to_string()
                                ]
                            ),
                            (
                                "command2".to_string(),
                                vec!["a".to_string(), "b".to_string(), "c".to_string(),]
                            )
                        ],
                        vec![
                            (
                                "cm1".to_string(),
                                vec!["a".to_string(), "b".to_string(), "c".to_string(),]
                            ),
                            (
                                "cm2".to_string(),
                                vec!["a".to_string(), "b".to_string(), "c".to_string(),]
                            )
                        ],
                        vec![
                            UnitTest::new_dummy(1)
                                .build_grading_unit_test(1, &executables_by_name)
                                .unwrap(),
                            UnitTest::new_dummy(2)
                                .build_grading_unit_test(1, &executables_by_name)
                                .unwrap(),
                            UnitTest::new_dummy(1)
                                .build_grading_unit_test(1, &executables_by_name)
                                .unwrap(),
                        ]
                    )
                );
            }
        }
    }
}
