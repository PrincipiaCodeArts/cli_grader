use crate::{
    config::{
        DEFAULT_MAIN_PROGRAM_NAME1, DEFAULT_MAIN_PROGRAM_NAME2, DEFAULT_PREFIX_PROGRAM_NAME1,
        DEFAULT_PREFIX_PROGRAM_NAME2,
    },
    input::ProgramType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone, Copy)]
pub enum InputType {
    #[default]
    #[serde(rename = "exe")]
    CompiledProgram,
}

impl From<InputType> for ProgramType {
    fn from(val: InputType) -> Self {
        match val {
            InputType::CompiledProgram => ProgramType::Compiled,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
pub enum ProgramSpecification {
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

impl ProgramSpecification {
    fn get_program_type(&self) -> InputType {
        match self {
            ProgramSpecification::OnlyType(input_type) => *input_type,
            ProgramSpecification::Complete {
                alias: _,
                program_type,
            } => *program_type,
        }
    }
}

impl Default for ProgramSpecification {
    fn default() -> Self {
        Self::OnlyType(InputType::default())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct InputSectionUnchecked {
    input_programs: Option<Vec<ProgramSpecification>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "InputSectionUnchecked")]
pub struct InputSection {
    /// This vector will define all the programs that will be available in the scope of the
    /// test.
    ///
    /// # Default
    /// Defaults to only one program with the standard name: "program1" and one additional  "p1"
    input_programs: Vec<ProgramSpecification>,

    // aux
    /// This field maps each possible program name to its relative index in
    /// `input_programs`.
    #[serde(skip)]
    program_name_by_index: HashMap<String, usize>,
}

impl InputSection {
    pub fn build(input_programs: Vec<ProgramSpecification>) -> Result<Self, &'static str> {
        if input_programs.is_empty() {
            return Err("input_program array may not be empty");
        }
        let len = input_programs.len();
        let mut program_name_to_index = HashMap::with_capacity(len * 3);

        // First, add all the default aliases, which have priority
        for i in 0..len {
            let j = i + 1;
            program_name_to_index.insert(format!("{DEFAULT_PREFIX_PROGRAM_NAME1}{j}"), i);
            program_name_to_index.insert(format!("{DEFAULT_PREFIX_PROGRAM_NAME2}{j}"), i);
        }

        // Then, add aliases user defined aliases
        for (i, input_program) in input_programs.iter().enumerate().take(len) {
            if let ProgramSpecification::Complete {
                alias,
                program_type: _,
            } = input_program
            {
                if program_name_to_index.contains_key(alias) {
                    return Err("duplicated alias (<alias>)");
                }
                program_name_to_index.insert(alias.clone(), i);
            }
        }

        Ok(InputSection {
            input_programs,
            program_name_by_index: program_name_to_index,
        })
    }

    /// Whether this input section has the `program_name` available in its name space.
    pub fn contains_program_name(&self, program_name: &str) -> bool {
        self.program_name_by_index.contains_key(program_name)
    }

    pub fn get_program_index_unchecked(&self, program_name: &str) -> usize {
        self.program_name_by_index[program_name]
    }

    pub fn get_program_type_unchecked(&self, program_name: &str) -> InputType {
        let i = self.program_name_by_index[program_name];
        self.input_programs[i].get_program_type()
    }

    pub fn input_programs_size(&self) -> usize {
        self.input_programs.len()
    }

    pub fn get_program_name_by_index(&self) -> &HashMap<String, usize> {
        &self.program_name_by_index
    }
}

impl Default for InputSection {
    fn default() -> Self {
        Self {
            input_programs: vec![ProgramSpecification::default()],
            program_name_by_index: HashMap::from_iter([
                (DEFAULT_MAIN_PROGRAM_NAME1.to_string(), 0),
                (DEFAULT_MAIN_PROGRAM_NAME2.to_string(), 0),
            ]),
        }
    }
}

impl TryFrom<InputSectionUnchecked> for InputSection {
    type Error = &'static str;

    fn try_from(value: InputSectionUnchecked) -> Result<Self, Self::Error> {
        let InputSectionUnchecked { input_programs } = value;
        match input_programs {
            Some(input_programs) => InputSection::build(input_programs),
            None => Ok(InputSection::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod test_program_specification {
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

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
        use super::*;
        use crate::config::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

        // serialization
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
                ],
                program_name_by_index: HashMap::from_iter([
                    // p1
                    (DEFAULT_MAIN_PROGRAM_NAME1.to_string(), 0),
                    (DEFAULT_MAIN_PROGRAM_NAME2.to_string(), 0),
                    // p2
                    (format!("{DEFAULT_PREFIX_PROGRAM_NAME1}2"), 1),
                    (format!("{DEFAULT_PREFIX_PROGRAM_NAME2}2"), 1),
                    ("hello".to_string(), 1),
                    // p3
                    (format!("{DEFAULT_PREFIX_PROGRAM_NAME1}3"), 2),
                    (format!("{DEFAULT_PREFIX_PROGRAM_NAME2}3"), 2),
                ])
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
        test_invalid_deserialization!(
            should_panic_empty_input_programs,
            r#"
        {
            "input_programs": []
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_invalid_alias,
            r#"
        {
            "input_programs": ["exe", {"program_type":"exe", "alias":"p2"}]
        }"#,
            InputSection
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_alias,
            r#"
        {
            "input_programs": [
                "exe", 
                {"program_type":"exe", "alias":"java"},
                {"program_type":"exe", "alias":"python"},
                {"program_type":"exe", "alias":"java"}
            ]
        }"#,
            InputSection
        );

        // valid
        test_valid_deserialization!(should_accept_empty, r#"{}"#, InputSection);
        test_valid_deserialization!(
            should_accept_with_input_programs,
            r#"
        {
            "input_programs": ["exe", {"program_type":"exe", "alias":"programB"}]
        }"#,
            InputSection
        );
    }
}
