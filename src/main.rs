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

    let mut agent = match Agent::new(base_url, token).await {
        Ok(a) => a,
        Err(error) => {
            error!("{}", error);
            return Err(error.into());
        }
    };

    let agent_json = serde_json::to_string_pretty(&agent).unwrap();

    debug!("Current Agent: {}", agent_json);

    agent.register().await?;

    agent.submit_capabilities().await?;

    // TODO: handle errors not related to JobFailed
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    while !term.load(Ordering::Relaxed) {
        agent.announce_presence().await?;
        agent.get_jobs().await?;

        agent.run_jobs().await?;

        agent.submit_report().await?;

        sleep(Duration::from_secs(args.refresh_timeout)).await;
    }

    Ok(())
}
