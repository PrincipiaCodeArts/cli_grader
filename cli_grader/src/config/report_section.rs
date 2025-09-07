use crate::report::ReportOutput;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ReportSection {
    is_verbose: bool,
    output: ReportOutput,
}

impl ReportSection {
    pub fn new(is_verbose: bool, output: ReportOutput) -> Self {
        Self { is_verbose, output }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_macros::{
        test_invalid_deserialization, test_serialize_and_deserialize, test_valid_deserialization,
    };

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
