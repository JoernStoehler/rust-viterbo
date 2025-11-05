use anyhow::Result;
use clap::{Parser, Subcommand};
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
        days: u32
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
    // Stub: read input via Polars if needed; call into `core`.
    std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap())?;
    std::fs::write(&out, b"{}")?;
    Ok(())
}

fn figure(from: String, out: String) -> Result<()> {
    tracing::info!(from, out, "figure");
    std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap())?;
    std::fs::write(&out, b"[]")?;
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

