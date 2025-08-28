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

mod agent;
mod api;
mod job;
mod tool;

use crate::{agent::Agent, api::client::ClientError};

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
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);

    let args = Args::parse();

    let base_url = args.api_url;
    let token = args.auth_token.to_string();

    let mut agent = match Agent::new(base_url, token).await {
        Ok(a) => a,
        Err(error) => {
            info!("Error: {}", error);
            return Err(error.into()); // or just return Err(error) depending on your fn signature
        }
    };

    let agent_json = serde_json::to_string_pretty(&agent).unwrap();

    debug!("Current Agent: {}", agent_json);

    // TODO: implement: fetch hostname and platform
    // agent.register().await?;
    debug!("Submitting submit_capabilities...");
    agent.submit_capabilities().await?;
    debug!("Finished!");

    agent.run_jobs().await?;

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
