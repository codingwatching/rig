use rig::prelude::*;
use rig::{
    agent::AgentBuilder,
    completion::{Prompt, ToolDefinition},
    providers::{self, huggingface::SubProvider},
    tool::Tool,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create streaming agent with a single context prompt
    let models = [
        ("deepseek-ai/DeepSeek-V3", SubProvider::Together),
        ("meta-llama/Llama-3.1-8B-Instruct", SubProvider::HFInference),
        ("Meta-Llama-3.1-8B-Instruct", SubProvider::SambaNova),
        ("deepseek-v3", SubProvider::Fireworks),
        ("Qwen/Qwen2.5-32B-Instruct", SubProvider::Nebius),
    ];
    for (model, sub_provider) in models {
        tools(model, sub_provider).await?;
    }
    Ok(())
}

fn client(sub_provider: SubProvider) -> providers::huggingface::Client {
    let api_key = &env::var("HUGGINGFACE_API_KEY").expect("HUGGINGFACE_API_KEY not set");
    providers::huggingface::ClientBuilder::new(api_key)
        .sub_provider(sub_provider)
        .build()
        .expect("Failed to build client")
}

/// Create a partial huggingface agent (deepseek R1)
fn partial_agent(
    model: &str,
    sub_provider: SubProvider,
) -> AgentBuilder<providers::huggingface::completion::CompletionModel> {
    let client = client(sub_provider);
    client.agent(model)
}

/// Create an huggingface agent (deepseek R1) with tools
/// Based upon the `tools` example
///
/// This example creates a calculator agent with two tools: add and subtract
async fn tools(model: &str, sub_provider: SubProvider) -> Result<(), anyhow::Error> {
    // Create agent with a single context prompt and two tools
    let calculator_agent = partial_agent(model, sub_provider.clone())
        .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)
        .build();
    // Prompt the agent and print the response
    println!("Asking {model} on {sub_provider:?} to Calculate 2 - 5");
    println!(
        "Calculator Agent: {}",
        calculator_agent.prompt("Calculate 2 - 5").await?
    );
    Ok(())
}

#[derive(Deserialize)]
struct OperationArgs {
    x: f32,
    y: f32,
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
    type Output = f32;
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
    type Output = f32;

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
