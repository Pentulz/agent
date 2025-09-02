use clap::Parser;
use spdlog::prelude::*;
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time::sleep;

mod action;
mod agent;
mod api;
mod job;
mod tool;

use crate::agent::Agent;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    token: String,

    #[arg(long)]
    api_url: String,

    #[arg(long)]
    refresh_timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);

    let args = Args::parse();

    let base_url = args.api_url;
    let token = args.token.to_string();

    debug!("Creating agent...");

    let mut agent = match Agent::new(base_url, token).await {
        Ok(a) => a,
        Err(error) => {
            error!("Error: {}", error);
            return Err(error.into());
        }
    };

    let agent_json = serde_json::to_string_pretty(&agent).unwrap();

    debug!("Current Agent: {}", agent_json);

    debug!("Registring agent...");
    agent.register().await?;
    debug!("Finished!");

    debug!("Submitting submit_capabilities...");
    agent.submit_capabilities().await?;
    debug!("Finished!");

    // TODO: handle errors not related to JobFailed
    let _ = agent.run_jobs().await;
    debug!("Submitting job report...");
    agent.submit_report().await?;
    debug!("Finished!");
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    while !term.load(Ordering::Relaxed) {
        agent.check_health().await?;

        debug!("Fetching jobs...");
        agent.get_jobs().await?;
        debug!("Finished");

        debug!("Running jobs...");
        agent.run_jobs().await?;
        debug!("Finished");

        sleep(Duration::from_secs(args.refresh_timeout)).await;
    }

    Ok(())
}
