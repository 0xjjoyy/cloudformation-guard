// © Amazon Web Services, Inc. or its affiliates. All Rights Reserved. This AWS Content is provided subject to the terms of the AWS Customer Agreement available at http://aws.amazon.com/agreement or other written agreement between Customer and either Amazon Web Services, Inc. or Amazon Web Services EMEA SARL or both.
// Structs, Enums and Impls

pub mod enums {
    #[derive(Debug)]
    pub enum LineType {
        Assignment,
        Comment,
        Rule,
    }

    #[derive(Debug)]
    #[derive(Hash)]
    #[derive(PartialEq, Eq)]
    #[derive(Clone)]
    pub enum OpCode {
        Require,
        RequireNot,
        In,
        NotIn,
    }

    #[derive(Debug)]
    #[derive(Hash)]
    #[derive(PartialEq, Eq)]
    #[derive(Clone)]
    pub enum RValueType {
        Value,
        List,
        Regex,
        Variable,
    }
    #[derive(Debug)]
    #[derive(Clone)]
    pub enum CompoundType {
        OR,
        AND,
    }
}

pub mod structs {
    use std::collections::HashMap;

    #[derive(Debug)]
    #[derive(Hash)]
    #[derive(Eq)]
    #[derive(Clone)]
    pub struct Rule {
        pub(crate) resource_type: String,
        pub(crate) field: String,
        pub(crate) operation: super::enums::OpCode,
        pub(crate) value: String,
        pub(crate) rule_vtype: super::enums::RValueType,
        pub(crate) custom_msg: Option<String>,
    }

    impl PartialEq for Rule {
        fn eq(&self, other: &Rule) -> bool {
            let self_string: String = format!("{:#?}", self);
            let other_string: String = format!("{:#?}", other);
            self_string == other_string
        }
    }

    #[derive(Debug)]
    pub struct CompoundRule {
        pub(crate) compound_type: super::enums::CompoundType,
        pub(crate) rule_list: Vec<Rule>,
    }

    #[derive(Debug)]
    pub struct ParsedRuleSet {
        pub(crate) variables: HashMap<String, String>,
        pub(crate) rule_set: Vec<CompoundRule>,
    }
}
