use anyhow::Result;
use clap::{Parser, Subcommand};
use polars::prelude::*;
use std::path::Path;
use tracing_subscriber::fmt::SubscriberBuilder;

#[derive(Parser)]
#[command(name = "cli")]
#[command(about = "Orchestration and experiment runner")]
struct Cmd {
    /// Optional VK ticket UUID; propagated to outputs and logs
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

    // Write provenance.json next to the output (per AGENTS conventions)
    let rev = option_env!("GIT_COMMIT").unwrap_or("unknown");
    let provenance = serde_json::json!({
        "code_rev": rev,
        "vk": vk,
        "th": [],
        "params": {
            "algo": algo,
            "input": input,
            "input_head_shape": shape_opt
        },
        "outputs": [out]
    });
    let prov_path = out_path.with_file_name("provenance.json");
    std::fs::write(prov_path, serde_json::to_vec_pretty(&provenance)?)?;

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

    // Write provenance next to figure output as well
    let rev = option_env!("GIT_COMMIT").unwrap_or("unknown");
    let provenance = serde_json::json!({
        "code_rev": rev,
        "vk": null,
        "th": [],
        "params": {
            "from": from
        },
        "outputs": [out]
    });
    let prov_path = out_path.with_file_name("provenance.json");
    std::fs::write(prov_path, serde_json::to_vec_pretty(&provenance)?)?;
    Ok(())
}

fn report(vk: Option<String>) -> Result<()> {
    let rev = option_env!("GIT_COMMIT").unwrap_or("unknown");
    let obj = serde_json::json!({
        "code_rev": rev,
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
