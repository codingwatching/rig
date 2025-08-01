use rig::prelude::*;
use std::env;

use rig::{
    agent::AgentBuilder,
    completion::{Prompt, ToolDefinition},
    loaders::FileLoader,
    providers,
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Runs 4 agents based on deepseek R1 (derived from the other examples)
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Running basic agent with deepseek R1");
    basic().await?;

    println!("\nRunning deepseek R1 agent with tools");
    tools().await?;

    println!("\nRunning deepseek R1 agent with loaders");
    loaders().await?;

    println!("\nRunning deepseek R1 agent with context");
    context().await?;

    println!("\n\nAll agents ran successfully");
    Ok(())
}

fn client() -> providers::huggingface::Client {
    let api_key = &env::var("HUGGINGFACE_API_KEY").expect("HUGGINGFACE_API_KEY not set");
    providers::huggingface::Client::new(api_key)
}

/// Create a partial huggingface agent (deepseek R1)
fn partial_agent() -> AgentBuilder<providers::huggingface::completion::CompletionModel> {
    let client = client();
    client.agent("deepseek-ai/DeepSeek-R1-Distill-Qwen-32B")
}

/// Create an huggingface agent (deepseek R1) with a preamble, based upon the `agent` example.
/// This example creates a comedian agent with a preamble
async fn basic() -> Result<(), anyhow::Error> {
    let comedian_agent = partial_agent()
        .preamble("You are a comedian here to entertain the user using humour and jokes.")
        .build();

    // Prompt the agent and print the response

    let response = comedian_agent.prompt("Entertain me!").await?;

    println!("{response}");

    Ok(())
}

/// Create an huggingface agent (deepseek R1) with tools, based upon the `tools` example.
/// This example creates a calculator agent with two tools: add and subtract
async fn tools() -> Result<(), anyhow::Error> {
    // Create agent with a single context prompt and two tools
    let calculator_agent = partial_agent()
        .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)
        .build();
    // Prompt the agent and print the response
    println!("Calculate 2 - 5");
    println!(
        "Calculator Agent: {}",
        calculator_agent.prompt("Calculate 2 - 5").await?
    );
    Ok(())
}

/// Create an huggingface agent (deepseek R1) with loaders, based upon the `loaders` example
/// This example loads in all the rust examples from the rig-core crate and uses them as
/// context for the agent
async fn loaders() -> Result<(), anyhow::Error> {
    let model = client().completion_model("deepseek-ai/DeepSeek-R1-Distill-Qwen-32B");
    // Load in all the rust examples
    let examples = FileLoader::with_glob("rig-core/examples/*.rs")?
        .read_with_path()
        .ignore_errors()
        .into_iter()
        .step_by(2);
    // Create an agent with multiple context documents
    let agent = examples
        .fold(AgentBuilder::new(model), |builder, (path, content)| {
            builder.context(format!("Rust Example {path:?}:\n{content}").as_str())
        })
        .build();
    // Prompt the agent and print the response
    let response = agent
        .prompt("Which rust example is best suited for the operation 1 + 2")
        .await?;

    println!("{response}");

    Ok(())
}

async fn context() -> Result<(), anyhow::Error> {
    let model = client().completion_model("deepseek-ai/DeepSeek-R1-Distill-Qwen-32B");
    // Create an agent with multiple context documents
    let agent = AgentBuilder::new(model)
        .context("Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets")
        .context("Definition of a *glarb-glarb*: A glarb-glarb is an ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.")
        .context("Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.")
        .build();
    // Prompt the agent and print the response
    let response = agent.prompt("What does \"glarb-glarb\" mean?").await?;

    println!("{response}");

    Ok(())
}

#[derive(Deserialize)]
struct OperationArgs {
    x: i32,
    y: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The first number to add"
                    },
                    "y": {
                        "type": "number",
                        "description": "The second number to add"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = args.x + args.y;
        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
struct Subtract;
impl Tool for Subtract {
    const NAME: &'static str = "subtract";
    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "subtract",
            "description": "Subtract y from x (i.e.: x - y)",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The number to subtract from"
                    },
                    "y": {
                        "type": "number",
                        "description": "The number to subtract"
                    }
                }
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = args.x - args.y;
        Ok(result)
    }
}
