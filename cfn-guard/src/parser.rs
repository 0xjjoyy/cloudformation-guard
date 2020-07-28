// © Amazon Web Services, Inc. or its affiliates. All Rights Reserved. This AWS Content is provided subject to the terms of the AWS Customer Agreement available at http://aws.amazon.com/agreement or other written agreement between Customer and either Amazon Web Services, Inc. or Amazon Web Services EMEA SARL or both.

use std::collections::{HashMap, HashSet};
use std::env;
use std::process;

use log::{self, debug, error, trace};
use regex::{Captures, Regex};
use serde_json::Value;

use crate::guard_types::enums::{CompoundType, LineType, OpCode, RValueType, RuleType};
use crate::guard_types::structs::{CompoundRule, ConditionalRule, ParsedRuleSet, Rule};
use crate::util;
use lazy_static::lazy_static;

// This sets it up so the regexen only get compiled once
// See: https://docs.rs/regex/1.3.9/regex/#example-avoid-compiling-the-same-regex-in-a-loop
lazy_static! {
    static ref ASSIGN_REG: Regex = Regex::new(r"let (?P<var_name>\w+) +(?P<operator>\S+) +(?P<var_value>.+)").unwrap();
    static ref RULE_REG: Regex = Regex::new(r"(?P<resource_type>\S+) +(?P<resource_property>[\w\.\*]+) +(?P<operator>\S+) +(?P<rule_value>[^\n\r]+)").unwrap();
    static ref COMMENT_REG: Regex = Regex::new(r#"#(?P<comment>.*)"#).unwrap();
    static ref WILDCARD_OR_RULE_REG: Regex = Regex::new(r"(\S+) (\S+\*\S*) (==|IN) (.+)").unwrap();
    static ref RULE_WITH_OPTIONAL_MESSAGE_REG: Regex = Regex::new(
        r"(?P<resource_type>\S+) +(?P<resource_property>[\w\.\*]+) +(?P<operator>\S+) +(?P<rule_value>[^\n\r]+) +<{2} *(?P<custom_msg>.*)").unwrap();
    static ref WHITE_SPACE_REG: Regex = Regex::new(r"\s+").unwrap();
    static ref CONDITIONAL_RULE_REG: Regex = Regex::new(r"(?P<resource_type>\S+) +if +(?P<condition>.+) +then +(?P<consequent>.*)").unwrap();
}

pub(crate) fn parse_rules(
    rules_file_contents: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<ParsedRuleSet, String> {
    debug!("Entered parse_rules");
    trace!(
        "Parse rules entered with rules_file_contents: {:#?}",
        &rules_file_contents
    );
    trace!(
        "Parse rules entered with cfn_resources: {:#?}",
        &cfn_resources
    );

    let mut rule_set: Vec<RuleType> = vec![];
    let mut variables = HashMap::new();

    let lines = rules_file_contents.lines();
    trace!("Rules file lines: {:#?}", &lines);

    for l in lines {
        debug!("Parsing '{}'", &l);
        if l.is_empty() {
            continue;
        };
        let line_type = find_line_type(l);
        debug!("line_type is {:#?}", line_type);
        match line_type {
            LineType::Assignment => {
                let caps = match process_assignment(l) {
                    Ok(a) => a,
                    Err(e) => return Err(e),
                };
                trace!("Parsed assignment's captures are: {:#?}", &caps);
                if caps["operator"] != *"=" {
                    let msg_string = format!(
                        "Bad Assignment Operator: [{}] in '{}'",
                        &caps["operator"], l
                    );
                    error!("{}", &msg_string);
                    process::exit(1)
                }
                let var_name = caps["var_name"].to_string();
                let var_value = caps["var_value"].to_string();
                trace!(
                    "Inserting key: [{}], value: [{}] into variables",
                    var_name,
                    var_value
                );
                variables.insert(var_name, var_value);
            }
            LineType::Comment => (),
            LineType::Rule => match parse_rule_line(l, &cfn_resources) {
                Ok(prl) => {
                    debug!("Parsed rule is: {:#?}", &prl);
                    rule_set.push(RuleType::CompoundRule(prl))
                }
                Err(e) => return Err(e),
            },
            LineType::Conditional => match process_conditional(l, &cfn_resources) {
                Ok(c) => {
                    debug!("Parsed conditional is {:#?}", &c);
                    rule_set.push(RuleType::ConditionalRule(c));
                }
                Err(e) => return Err(e),
            },
            LineType::WhiteSpace => {
                debug!("Line is white space");
                continue;
            }
        }
    }
    for (key, value) in env::vars() {
        let key_name = format!("ENV_{}", key);
        variables.insert(key_name, value);
    }
    let filtered_env_vars = util::filter_for_env_vars(&variables);
    debug!("Variables dictionary is {:?}", &filtered_env_vars);
    debug!("Rule Set is {:#?}", &rule_set);
    Ok(ParsedRuleSet {
        variables,
        rule_set,
    })
}

fn parse_rule_line(
    l: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<CompoundRule, String> {
    match is_or_rule(l) {
        true => {
            debug!("Line is an |OR| rule");
            match process_or_rule(l, &cfn_resources) {
                Ok(r) => Ok(r),
                Err(e) => Err(e),
            }
        }
        false => {
            debug!("Line is an 'AND' rule");
            match process_and_rule(l, &cfn_resources) {
                Ok(r) => Ok(r),
                Err(e) => return Err(e),
            }
        }
    }
}

fn process_conditional(
    line: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<ConditionalRule, String> {
    let caps = CONDITIONAL_RULE_REG.captures(line).unwrap();
    trace!("ConditionalRule regex captures are {:#?}", &caps);

    let conjd_caps_conditional = format!("{} {}", &caps["resource_type"], &caps["condition"]);
    trace!("conjd_caps_conditional is {:#?}", conjd_caps_conditional);
    match parse_rule_line(&conjd_caps_conditional, cfn_resources) {
        Ok(condition) => {
            let conjd_caps_consequent =
                format!("{} {}", &caps["resource_type"], &caps["consequent"]);
            trace!("conjd_caps_consequent is {:#?}", conjd_caps_consequent);
            match parse_rule_line(&conjd_caps_consequent, cfn_resources) {
                Ok(consequent) => Ok(ConditionalRule {
                    condition,
                    consequent,
                }),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
    // if let condition = parse_rule_line(&conjd_caps_conditional, cfn_resources)
    //     .unwrap_or_else(|err| return Err(err));
    //
    // let conjd_caps_consequent = format!("{} {}", &caps["resource_type"], &caps["consequent"]);
    // trace!("conjd_caps_consequent is {:#?}", conjd_caps_consequent);
    // if let consequent = parse_rule_line(&conjd_caps_consequent, cfn_resources)
    //     .unwrap_or_else(|err| return Err(err));
    //
    // ok(conditionalrule {
    //     condition,
    //     consequent,
    // })
}

fn find_line_type(line: &str) -> LineType {
    if COMMENT_REG.is_match(line) {
        return LineType::Comment;
    };
    if ASSIGN_REG.is_match(line) {
        return LineType::Assignment;
    };
    if CONDITIONAL_RULE_REG.is_match(line) {
        return LineType::Conditional;
    };
    if RULE_REG.is_match(line) {
        return LineType::Rule;
    };
    if WHITE_SPACE_REG.is_match(line) {
        return LineType::WhiteSpace;
    }
    let msg_string = format!("BAD RULE: {:?}", line);
    error!("{}", &msg_string);
    process::exit(1)
}

fn process_assignment(line: &str) -> Result<Captures, String> {
    match ASSIGN_REG.captures(line) {
        Some(c) => Ok(c),
        None => Err(format!("Invalid assignment statement: '{}", line)),
    }
}

fn is_or_rule(line: &str) -> bool {
    line.contains("|OR|") || WILDCARD_OR_RULE_REG.is_match(line)
}

fn process_or_rule(
    line: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<CompoundRule, String> {
    trace!("Entered process_or_rule");
    let branches = line.split("|OR|");
    debug!("Rule branches are: {:#?}", &branches);
    let mut rules: Vec<Rule> = vec![];
    for b in branches {
        match destructure_rule(b.trim(), cfn_resources) {
            Ok(r) => rules.append(&mut r.clone()),
            Err(e) => return Err(e),
        }
    }
    Ok(CompoundRule {
        compound_type: CompoundType::OR,
        raw_rule: line.to_string(),
        rule_list: rules,
    })
}

fn process_and_rule(
    line: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<CompoundRule, String> {
    match destructure_rule(line, cfn_resources) {
        Ok(r) => Ok(CompoundRule {
            compound_type: CompoundType::AND,
            raw_rule: line.to_string(),
            rule_list: r,
        }),
        Err(e) => Err(e),
    }
}

fn destructure_rule(
    rule_text: &str,
    cfn_resources: &HashMap<String, Value>,
) -> Result<Vec<Rule>, String> {
    trace!("Entered destructure_rule");
    let mut rules_hash: HashSet<Rule> = HashSet::new();
    let caps = match RULE_WITH_OPTIONAL_MESSAGE_REG.captures(rule_text) {
        Some(c) => c,
        None => match RULE_REG.captures(rule_text) {
            Some(c) => c,
            None => {
                return Err(format!("Invalid rule: {}", rule_text));
            }
        },
    };

    trace!("Parsed rule's captures are: {:#?}", &caps);
    let mut props: Vec<String> = vec![];
    if caps["resource_property"].contains('*') {
        for (_name, value) in cfn_resources {
            if caps["resource_type"] == value["Type"] {
                if let Some(p) = util::expand_wildcard_props(
                    &value["Properties"],
                    caps["resource_property"].to_string(),
                    String::from(""),
                ) {
                    props.append(&mut p.clone());
                    trace!("Expanded props are {:#?}", &props);
                }
            }
        }
    } else {
        props.push(caps["resource_property"].to_string());
    };

    for p in props {
        rules_hash.insert(Rule {
            resource_type: caps["resource_type"].to_string(),
            field: p.to_string(),
            operation: {
                match &caps["operator"] {
                    "==" => OpCode::Require,
                    "!=" => OpCode::RequireNot,
                    "<" => OpCode::LessThan,
                    ">" => OpCode::GreaterThan,
                    "<=" => OpCode::LessThanOrEqualTo,
                    ">=" => OpCode::GreaterThanOrEqualTo,
                    "IN" => OpCode::In,
                    "NOT_IN" => OpCode::NotIn,
                    _ => {
                        let msg_string = format!(
                            "Bad Rule Operator: [{}] in '{}'",
                            &caps["operator"], rule_text
                        );
                        error!("{}", &msg_string);
                        process::exit(1)
                    }
                }
            },
            rule_vtype: {
                let rv = caps["rule_value"].chars().next().unwrap();
                match rv {
                    '[' => match &caps["operator"] {
                        "==" | "!=" | "<=" | ">=" | "<" | ">" => RValueType::Value,
                        "IN" | "NOT_IN" => RValueType::List,
                        _ => {
                            let msg_string = format!(
                                "Bad Rule Operator: [{}] in '{}'",
                                &caps["operator"], rule_text
                            );
                            error!("{}", &msg_string);
                            process::exit(1)
                        }
                    },
                    '/' => RValueType::Regex,
                    '%' => RValueType::Variable,
                    _ => RValueType::Value,
                }
            },
            value: {
                let rv = caps["rule_value"].chars().next().unwrap();
                match rv {
                    '/' => caps["rule_value"].trim_matches('/').to_string(),
                    _ => caps["rule_value"].to_string().trim().to_string(),
                }
            },
            custom_msg: match caps.name("custom_msg") {
                Some(s) => Some(s.as_str().to_string()),
                None => None,
            },
        });
    }

    let rules = rules_hash.into_iter().collect::<Vec<Rule>>();
    trace!("Destructured rules are: {:#?}", &rules);
    Ok(rules)
}

mod tests {
    #[cfg(test)]
    use super::*;

    #[test]
    fn test_find_line_type() {
        let comment = find_line_type("# This is a comment");
        let assignment = find_line_type("let x = assignment");
        let rule = find_line_type("AWS::EC2::Volume Encryption == true");
        let white_space = find_line_type("         ");
        assert_eq!(comment, crate::enums::LineType::Comment);
        assert_eq!(assignment, crate::enums::LineType::Assignment);
        assert_eq!(rule, crate::enums::LineType::Rule);
        assert_eq!(white_space, crate::enums::LineType::WhiteSpace)
    }

    #[test]
    fn test_parse_variable() {
        let assignment = "let var = [128]";
        let cfn_resources: HashMap<String, Value> = HashMap::new();
        let mut var_map: HashMap<String, String> = HashMap::new();
        var_map.insert("var".to_string(), "[128]".to_string());

        let parsed_rules = parse_rules(assignment, &cfn_resources).unwrap();
        assert!(parsed_rules.variables["var"] == "[128]");
    }
}
