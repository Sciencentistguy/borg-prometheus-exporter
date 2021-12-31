mod config;

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::{fmt::Write, path::Path};

use chrono::{Local, NaiveDateTime, TimeZone};
use eyre::Context;
use eyre::Result;
use once_cell::sync::Lazy;
use clap::Parser;
use tracing::Level;
use tracing::*;
use warp::Filter;

use crate::config::Config;

static OPTIONS: Lazy<Opt> = Lazy::new(Opt::parse);
static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::open_or_create().unwrap_or_else(|e| {
        error!(error = ?e, "An error ocurred while opening config file");
        panic!("{:?}", e);
    })
});

mod parse;

async fn read_repo_info(repo_path: &Path, writer: &mut impl Write) -> Result<()> {
    trace!(?repo_path, "Reading repo info");

    let repo_name = repo_path
        .file_name()
        .ok_or_else(|| eyre::eyre!("Invalid repo path: {:?}", repo_path))?
        .to_str()
        .ok_or_else(|| eyre::eyre!("Invalid utf-8 in repo name"))?;

    let mut command = Command::new("borg");
    command.arg("info").arg("--json").arg(repo_path.as_os_str());

    let stdout = loop {
        let output = command.output().wrap_err("Failed to run command")?;
        if output.status.success() {
            break output.stdout;
        } else if output
            .stderr
            .starts_with("Failed to create/acquire the lock".as_bytes())
        {
            warn!("The repo is busy. Retrying in 5s");
            tokio::time::sleep(Duration::from_secs(5)).await;
        } else {
            return Err(eyre::eyre!(
                "Borg info command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    };

    debug!(output = %std::str::from_utf8(&stdout)?, "`borg info` returned successfully");

    let repo_info: parse::BorgResponse = serde_json::from_slice(&stdout)
        .wrap_err("Failed to parse response from `borg info --json`")?;
    let trimmed_timestamp = &repo_info.repository.last_modified[..repo_info
        .repository
        .last_modified
        .find('.')
        .ok_or_else(|| {
            eyre::eyre!(
                "Expected a `.` in the timestamp: {}",
                repo_info.repository.last_modified
            )
        })?];

    let localtime = NaiveDateTime::parse_from_str(trimmed_timestamp, "%Y-%m-%dT%H:%M:%S")
        .wrap_err("Failed to parse timestamp")?;
    let actual_time = Local
        .from_local_datetime(&localtime)
        .single()
        .ok_or_else(|| eyre::eyre!("Failed to convert from local time to UTC"))?;

    let last_modified = actual_time.timestamp();

    debug!(?repo_info, %last_modified, "Parsed response");

    writeln!(writer, "# HELP borg_total_chunks borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_chunks gauge")?;
    writeln!(
        writer,
        "borg_total_chunks{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_chunks
    )?;

    writeln!(writer, "# HELP borg_total_csize borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_csize gauge")?;
    writeln!(
        writer,
        "borg_total_csize{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_csize
    )?;

    writeln!(writer, "# HELP borg_total_size borg-prometheus-exporter")?;
    writeln!(writer, "# TYPE borg_total_size gauge")?;
    writeln!(
        writer,
        "borg_total_size{{repository=\"{}\"}} {}",
        repo_name, repo_info.cache.stats.total_size
    )?;

    writeln!(
        writer,
        "# HELP borg_total_unique_chunks borg-prometheus-exporter"
    )?;
    writeln!(writer, "# TYPE borg_total_unique_chunks gauge")?;
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

    trace!("Wrote prometheus output");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::INFO)
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

        info!(%output, "Successfully generated prometheus output");

        Ok(output)
    });

    info!(port = %CONFIG.port, "Started listening");
    warp::serve(filter).run(([127, 0, 0, 1], CONFIG.port)).await;
    Ok(())
}

#[derive(Debug, Parser)]
struct Opt {
    /// The configuration file, in YAML
    config_file: PathBuf,
}
