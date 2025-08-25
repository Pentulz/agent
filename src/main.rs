use clap::Parser;
use std::{any::Any, error::Error};

mod agent;
mod api;
mod job;
mod tool;

use crate::agent::Agent;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    auth_token: String,

    #[arg(long)]
    api_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let base_url = args.api_url;
    let token = args.auth_token.to_string();

    let agent = Agent::new(base_url, token)?;
    agent.check_health().await?;

    println!("check_health: OK");

    Ok(())
}
