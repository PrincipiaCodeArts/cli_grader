use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub(crate) enum InputType {
    #[default]
    #[serde(rename = "exe")]
    CompiledProgram,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum ProgramSpecification {
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
struct InputSectionUnchecked {
    input_programs: Option<Vec<ProgramSpecification>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "InputSectionUnchecked")]
pub(crate) struct InputSection {
    /// This vector will define all the programs that will be available in the scope of the
    /// test.
    ///
    /// # Default
    /// Defaults to only one program with the standard name: "program1" and one additional  "p1"
    input_programs: Vec<ProgramSpecification>,
}

impl InputSection {
    pub(crate) fn available_program_names(&self) -> Result<HashSet<String>, String> {
        let mut r = HashSet::with_capacity(self.input_programs.len() * 2);
        for i in 0..self.input_programs.len() {
            r.insert(format!("p{}", i + 1));
            r.insert(format!("program{}", i + 1));
        }

        for p in self.input_programs.iter() {
            if let ProgramSpecification::Complete {
                alias,
                program_type: _,
            } = p
            {
                if r.insert(alias.clone()) {
                    continue;
                }
                return Err(format!("duplicated alias: \"{alias}\""));
            }
        }

        Ok(r)
    }
}

impl Default for InputSection {
    fn default() -> Self {
        Self {
            input_programs: vec![ProgramSpecification::default()],
        }
    }
}

impl TryFrom<InputSectionUnchecked> for InputSection {
    type Error = &'static str;

    fn try_from(value: InputSectionUnchecked) -> Result<Self, Self::Error> {
        let InputSectionUnchecked { input_programs } = value;
        match input_programs {
            Some(input_programs) if !input_programs.is_empty() => {
                Ok(InputSection { input_programs })
            }
            None => Ok(InputSection::default()),
            _ => Err("input_program array may not be empty"),
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
        test_invalid_deserialization!(
            should_panic_empty_input_programs,
            r#"
        {
            "input_programs": []
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
