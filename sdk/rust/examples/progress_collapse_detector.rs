use agentichound_trace::client::CollectorClient;
use agentichound_trace::contract_v0::TraceDocument;
use agentichound_trace::diagnostics::{diagnose_progress_collapse, ProgressCollapseDiagnostic};
use std::path::PathBuf;

#[derive(Debug)]
struct CliArgs {
    input: Option<PathBuf>,
    collector_url: Option<String>,
    run_id: Option<String>,
    json: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let trace = load_trace(&args).await?;
    let diagnostic = diagnose_progress_collapse(&trace);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&diagnostic)?);
    } else {
        print_text(&diagnostic);
    }

    Ok(())
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut input = None;
    let mut collector_url = None;
    let mut run_id = None;
    let mut json = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                let value = args.next().ok_or("--input requires a path")?;
                input = Some(PathBuf::from(value));
            }
            "--collector-url" => {
                let value = args.next().ok_or("--collector-url requires a URL")?;
                collector_url = Some(value);
            }
            "--run-id" => {
                let value = args.next().ok_or("--run-id requires a value")?;
                run_id = Some(value);
            }
            "--json" => json = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    Ok(CliArgs {
        input,
        collector_url,
        run_id,
        json,
    })
}

async fn load_trace(args: &CliArgs) -> Result<TraceDocument, Box<dyn std::error::Error>> {
    if let Some(input) = &args.input {
        let contents = std::fs::read_to_string(input)?;
        let trace = serde_json::from_str::<TraceDocument>(&contents)?;
        return Ok(trace);
    }

    let collector_url = args
        .collector_url
        .as_ref()
        .ok_or("either --input or --collector-url must be provided")?
        .clone();
    let run_id = args
        .run_id
        .as_ref()
        .ok_or("--collector-url requires --run-id")?
        .clone();
    let client = CollectorClient::new(collector_url)?;
    Ok(client.run(&run_id).await?.trace)
}

fn print_text(diagnostic: &ProgressCollapseDiagnostic) {
    println!("[AgenticHound Diagnostic] Progress Collapse Detector");
    println!("run_id: {}", diagnostic.run_id);
    println!("diagnostic: {}", diagnostic.diagnostic);
    println!("severity: {}", severity_label(diagnostic.severity));
    println!("duration_ms: {}", diagnostic.supporting_signals.duration_ms);
    println!(
        "spans: total={} non_retry={} retries={} errors={}",
        diagnostic.supporting_signals.span_count,
        diagnostic.supporting_signals.non_retry_span_count,
        diagnostic.supporting_signals.retry_span_count,
        diagnostic.supporting_signals.error_count
    );
    println!(
        "usage: total_tokens={} estimated_cost_usd={:.4}",
        diagnostic.supporting_signals.total_tokens, diagnostic.supporting_signals.total_cost_usd
    );
    println!("reasons:");
    for reason in &diagnostic.reasons {
        println!("- {reason}");
    }
    println!("summary:");
    println!("{}", diagnostic.summary);
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run --manifest-path sdk/rust/Cargo.toml --example progress_collapse_detector -- [--input PATH | --collector-url URL --run-id RUN_ID] [--json]"
    );
}

fn severity_label(severity: agentichound_trace::diagnostics::DiagnosticSeverity) -> &'static str {
    match severity {
        agentichound_trace::diagnostics::DiagnosticSeverity::Low => "low",
        agentichound_trace::diagnostics::DiagnosticSeverity::Medium => "medium",
        agentichound_trace::diagnostics::DiagnosticSeverity::High => "high",
        agentichound_trace::diagnostics::DiagnosticSeverity::Critical => "critical",
    }
}
