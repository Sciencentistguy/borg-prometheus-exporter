mod config;

use std::path::PathBuf;
use std::process::Command;
use std::{fmt::Write, path::Path};

use chrono::NaiveDateTime;
use eyre::Context;
use eyre::Result;
use once_cell::sync::Lazy;
use structopt::StructOpt;
use tracing::Level;
use tracing::*;
use warp::Filter;

use crate::config::Config;

static OPTIONS: Lazy<Opt> = Lazy::new(Opt::from_args);
static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::open_or_create().unwrap_or_else(|e| {
        error!(error = ?e, "An error ocurred while opening config file");
        panic!("{:?}", e);
    })
});

mod parse;

async fn read_repo_info(repo_path: &Path, writer: &mut impl Write) -> Result<()> {
    let repo_name = repo_path
        .file_name()
        .ok_or_else(|| eyre::eyre!("Invalid repo path: {:?}", repo_path))?
        .to_str()
        .ok_or_else(|| eyre::eyre!("Invalid utf-8 in repo name"))?;

    info!(?repo_path, "Reading repo info");
    let mut command = Command::new("borg");
    command.arg("info").arg("--json").arg(repo_path.as_os_str());
    let output = command.output().wrap_err("Failed to run command")?.stdout;
    let x = std::str::from_utf8(&output)?;

    info!(output = %x, "Got command output from borg");

    let repo_info: parse::BorgResponse = serde_json::from_slice(&output)
        .wrap_err("Failed to parse response from `borg info --json`")?;
    let trimmed_timestamp = &repo_info.repository.last_modified
        [..repo_info.repository.last_modified.find('.').unwrap()];

    info!(timestamp = %trimmed_timestamp);
    let last_modified = NaiveDateTime::parse_from_str(trimmed_timestamp, "%Y-%m-%dT%H:%M:%S")
        .wrap_err("Failed to parse timestamp")?
        .timestamp();

    writeln!(writer, "# HELP borg_total_chunks borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_chunks guage")?;
    writeln!(
        writer,
        "borg_total_chunks{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_chunks
    )?;

    writeln!(writer, "# HELP borg_total_csize borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_csize guage")?;
    writeln!(
        writer,
        "borg_total_csize{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_csize
    )?;

    writeln!(writer, "# HELP borg_total_size borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_size guage")?;
    writeln!(
        writer,
        "borg_total_size{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_size
    )?;

    writeln!(
        writer,
        "# HELP borg_total_unique_chunks borg-prometheus-exporter"
    )?;
    writeln!(writer, "# TYPE borg_total_unique_chunks guage")?;
    writeln!(
        writer,
        "borg_total_unique_chunks{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_unique_chunks
    )?;

    writeln!(writer, "# HELP borg_unique_csize borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_unique_csize gauge")?;
    writeln!(
        writer,
        "borg_unique_csize{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.unique_csize
    )?;

    writeln!(writer, "# HELP borg_unique_size borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_unique_size gauge")?;
    writeln!(
        writer,
        "borg_unique_size{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.unique_size
    )?;

    writeln!(writer, "# HELP borg_last_modified borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_last_modified counter")?;
    writeln!(
        writer,
        "borg_last_modified{{repository=\"{}\"}} {}",
        repo_name, last_modified
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .pretty()
            .finish(),
    )?;

    better_panic::install();

    Lazy::force(&CONFIG);

    let filter = warp::path("metrics").and_then(|| async move {
        info!("Received an incoming connection to `/metrics`");
        if false {
            // XXX: Type inference hint, I kinda hate it.
            return Err(warp::reject());
        }
        let mut output = String::new();
        for repo in &CONFIG.repositories {
            read_repo_info(repo, &mut output).await.map_err(|e| {
                error!(error = ?e, "An error ocurred in scrape_server");
                warp::reject::reject()
            })?;
        }

        Ok(output)
    });

    info!(port = %CONFIG.port, "Started listening.");
    warp::serve(filter).run(([127, 0, 0, 1], CONFIG.port)).await;
    Ok(())
}

#[derive(Debug, StructOpt)]
struct Opt {
    /// The configuration file, in YAML
    config_file: PathBuf,
}
