use rig::prelude::*;
use rig::{
    completion::Prompt,
    providers::anthropic::{self, CLAUDE_3_5_SONNET},
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create Anthropic client
    let client =
        anthropic::Client::new(&env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set"));

    // Create agent with a single context prompt
    let agent = client
        .agent(CLAUDE_3_5_SONNET)
        .preamble("Be precise and concise.")
        .temperature(0.5)
        .build();

    // Prompt the agent and print the response
    let response = agent
        .prompt("When and where and what type is the next solar eclipse?")
        .await?;

    println!("{response}");

    Ok(())
}
