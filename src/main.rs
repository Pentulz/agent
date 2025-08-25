use clap::Parser;
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time::sleep;

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

    #[arg(long)]
    refresh_timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let base_url = args.api_url;
    let token = args.auth_token.to_string();

    let agent = Agent::new(base_url, token)?;

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    while !term.load(Ordering::Relaxed) {
        agent.check_health().await?;

        println!("check_health: OK");

        sleep(Duration::from_secs(args.refresh_timeout)).await;
    }

    Ok(())
}
