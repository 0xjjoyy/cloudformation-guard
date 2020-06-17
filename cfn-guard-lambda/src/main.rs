// © Amazon Web Services, Inc. or its affiliates. All Rights Reserved. This AWS Content is provided subject to the terms of the AWS Customer Agreement available at http://aws.amazon.com/agreement or other written agreement between Customer and either Amazon Web Services, Inc. or Amazon Web Services EMEA SARL or both.

use std::error::Error;

use cfn_guard;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::{self, debug};
use serde_derive::{Deserialize, Serialize};
use simple_logger;

#[derive(Deserialize, Debug)]
struct CustomEvent {
    #[serde(rename = "template")]
    template: String,
    #[serde(rename = "ruleSet")]
    rule_set: String,
    strict_checks: bool
}

#[derive(Serialize)]
struct CustomOutput {
    message: Vec<String>,
    exit_status: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Debug)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: CustomEvent, _c: Context) -> Result<CustomOutput, HandlerError> {
    //dbg!(&e);
    debug!("Template is [{}]", &e.template);
    debug!("Rule Set is [{}]", &e.rule_set);
    let (result, exit_code) = cfn_guard::run_check(&e.template, &e.rule_set, e.strict_checks);

    let exit_status = match exit_code {
        0 => "PASS",
        1 => "ERR",
        2 | _ => "FAIL",
    };

    Ok(CustomOutput {
        message: result,
        exit_status: String::from(exit_status),
    })
}
