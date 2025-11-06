use anyhow::Result;
use clap::{Parser, Subcommand};
use polars::prelude::*;
use serde_json::json;
use std::path::Path;
use tracing_subscriber::fmt::SubscriberBuilder;

mod provenance;

#[derive(Parser)]
#[command(name = "cli")]
#[command(about = "Orchestration and experiment runner")]
struct Cmd {
    /// Optional VK ticket UUID; logged with tracing spans for easy correlation
    #[arg(long)]
    vk: Option<String>,

    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Run an algorithm and write heavy outputs under data/
    Run {
        #[arg(long)]
        algo: String,
        #[arg(long)]
        input: String,
        #[arg(long)]
        out: String,
    },
    /// Produce small publishable artifacts under docs/assets/
    Figure {
        #[arg(long)]
        from: String,
        #[arg(long)]
        out: String,
    },
    /// Print a small provenance JSON block
    Report,
    /// Clean old data/processed by age or tag (stub)
    Clean {
        #[arg(long, default_value_t = 30)]
        days: u32,
    },
}

fn main() -> Result<()> {
    SubscriberBuilder::default().with_target(false).init();
    let cmd = Cmd::parse();
    match cmd.action {
        Action::Run { algo, input, out } => run(algo, input, out, cmd.vk),
        Action::Figure { from, out } => figure(from, out),
        Action::Report => report(cmd.vk),
        Action::Clean { days } => clean(days),
    }
}

fn run(algo: String, input: String, out: String, vk: Option<String>) -> Result<()> {
    tracing::info!(algo, input, out, vk = ?vk, "run");
    // Minimal Polars usage: lazily read a CSV (head only) and log shape.
    let mut shape_opt: Option<(usize, usize)> = None;
    if input.ends_with(".csv") {
        let lf = LazyCsvReader::new(&input)
            .with_infer_schema_length(Some(100))
            .finish()?;
        let df = lf.limit(5).collect()?; // keep it light and fast
        shape_opt = Some(df.shape());
        tracing::info!(
            rows = df.height(),
            cols = df.width(),
            "input_csv_head_shape"
        );
    }

    // Write primary output
    let out_path = Path::new(&out);
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&out, b"{}")?;

    provenance::write_sidecar(
        out_path,
        provenance::Payload::new(json!({
            "algo": algo,
            "input": input,
            "input_head_shape": shape_opt,
        })),
    )?;

    Ok(())
}

fn figure(from: String, out: String) -> Result<()> {
    tracing::info!(from, out, "figure");
    let out_path = Path::new(&out);
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&out, b"[]")?;

    provenance::write_sidecar(
        out_path,
        provenance::Payload::new(json!({
            "from": from,
        })),
    )?;
    Ok(())
}

fn report(vk: Option<String>) -> Result<()> {
    let obj = json!({
        "code_rev": provenance::current_git_rev(),
        "vk": vk,
        "th": [],
        "params": {},
        "outputs": []
    });
    println!("{}", serde_json::to_string_pretty(&obj)?);
    Ok(())
}

fn clean(days: u32) -> Result<()> {
    tracing::info!(days, "clean");
    Ok(())
}
