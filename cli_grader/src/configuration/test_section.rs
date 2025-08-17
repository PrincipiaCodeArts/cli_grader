use crate::configuration::test_section::unit_tests::UnitTests;
use serde::{Deserialize, Serialize};

/*
mod performance_tests;
mod integration_tests;
 */

pub mod unit_tests;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct TestSectionUnchecked {
    title: Option<String>,
    weight: Option<u32>,
    unit_tests: Option<UnitTests>,
    // integration_tests: IntegrationTests,
    // performance_tests: PerformanceTests,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "TestSectionUnchecked")]
pub struct TestSection {
    pub title: Option<String>,
    pub weight: Option<u32>,
    /// # Caveats
    /// This field is optional for future purposes. In the future, it will be demanded that
    /// exactly one of `unit_tests`, `integration_tests`, or `performance_tests` is
    /// present.
    pub unit_tests: Option<UnitTests>,
    // integration_tests: IntegrationTests,
    // performance_tests: PerformanceTests,
}

impl TestSection {
    fn build(
        title: Option<String>,
        weight: Option<u32>,
        unit_tests: Option<UnitTests>,
    ) -> Result<Self, &'static str> {
        if unit_tests.is_none() {
            return Err("at least one type of test is expected in the TestSection");
        }

        Ok(Self {
            title,
            weight,
            unit_tests,
        })
    }
}

impl TryFrom<TestSectionUnchecked> for TestSection {
    type Error = &'static str;

    fn try_from(value: TestSectionUnchecked) -> Result<Self, Self::Error> {
        let TestSectionUnchecked {
            title,
            weight,
            unit_tests,
        } = value;

        TestSection::build(title, weight, unit_tests)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    mod test_test_section {
        use super::*;
        use crate::configuration::test_macros::{
            test_invalid_deserialization, test_serialize_and_deserialize,
            test_valid_deserialization,
        };

        // serialization
        test_serialize_and_deserialize!(
            should_serialize_deserialize_full,
            TestSection {
                title: Some("section 1".to_string()),
                weight: None,
                unit_tests: Some(UnitTests::new_dummy()),
            },
            TestSection
        );
        // invalid deserialization
        test_invalid_deserialization!(should_panic_with_no_content_string, r#"\n"#, TestSection);
        test_invalid_deserialization!(should_panic_with_empty_object, r#"{}"#, TestSection);
        test_invalid_deserialization!(
            should_panic_with_wrong_field,
            r#"
        {
            "wrong field":123
        }"#,
            TestSection
        );
        test_invalid_deserialization!(
            should_panic_without_tests,
            r#"
        {
            "title":"sec 1",
            "weight": 2
        }"#,
            TestSection
        );
        test_invalid_deserialization!(
            should_panic_with_extra_field,
            r#"
        {
            "title":"sec 1",
            "weight": 2,
            "extra":"field",
            "unit_tests":{
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
            }
        }
        "#,
            TestSection
        );
        test_invalid_deserialization!(
            should_panic_with_wrong_field_name,
            r#"
        {
            "title":"sec 1",
            "weigt": 2,
            "unit_tests":{
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
            }
        }
        "#,
            TestSection
        );
        test_invalid_deserialization!(
            should_panic_with_duplicated_field,
            r#"
        {
            "title":"sec 1",
            "weight": 2,
            "weight": 2,
            "unit_tests":{
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
            }
        }
        "#,
            TestSection
        );

        // valid deserialization
        test_valid_deserialization!(
            should_accept_valid_section,
            r#"
        {
            "title":"sec 1",
            "weight": 2,
            "unit_tests":{
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
            }
        }
        "#,
            TestSection
        );
    }
}
