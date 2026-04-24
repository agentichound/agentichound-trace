use agentichound_trace::client::CollectorClient;
use agentichound_trace::contract_v0::TraceDocument;
use agentichound_trace::diagnostics::{
    diagnose_progress_collapse, DiagnosticSeverity, ProgressCollapseDiagnostic,
};
use std::error::Error;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("diagnose") => run_diagnose(args.collect()).await,
        Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}").into()),
    }
}

async fn run_diagnose(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let parsed = parse_diagnose_args(&args)?;
    let trace = load_trace(&parsed).await?;
    let diagnostic = diagnose_progress_collapse(&trace);

    if parsed.json {
        println!("{}", serde_json::to_string_pretty(&diagnostic)?);
    } else {
        print_text(&diagnostic);
    }

    Ok(())
}

#[derive(Debug)]
struct DiagnoseArgs {
    input: Option<PathBuf>,
    collector_url: Option<String>,
    run_id: Option<String>,
    json: bool,
}

fn parse_diagnose_args(args: &[String]) -> Result<DiagnoseArgs, Box<dyn Error>> {
    let mut input = None;
    let mut collector_url = None;
    let mut run_id = None;
    let mut json = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                let value = iter
                    .next()
                    .ok_or_else(|| invalid_input("--input requires a path"))?;
                input = Some(PathBuf::from(value));
            }
            "--collector-url" => {
                let value = iter
                    .next()
                    .ok_or_else(|| invalid_input("--collector-url requires a URL"))?;
                collector_url = Some(value.clone());
            }
            "--run-id" => {
                let value = iter
                    .next()
                    .ok_or_else(|| invalid_input("--run-id requires a value"))?;
                run_id = Some(value.clone());
            }
            "--json" => json = true,
            "--help" | "-h" => {
                print_diagnose_help();
                std::process::exit(0);
            }
            other => {
                return Err(invalid_input(&format!("unknown diagnose argument: {other}")).into())
            }
        }
    }

    Ok(DiagnoseArgs {
        input,
        collector_url,
        run_id,
        json,
    })
}

async fn load_trace(args: &DiagnoseArgs) -> Result<TraceDocument, Box<dyn Error>> {
    if let Some(input) = &args.input {
        let contents = std::fs::read_to_string(input)?;
        return Ok(serde_json::from_str(&contents)?);
    }

    let collector_url = args
        .collector_url
        .as_ref()
        .ok_or_else(|| invalid_input("either --input or --collector-url must be provided"))?
        .clone();
    let run_id = args
        .run_id
        .as_ref()
        .ok_or_else(|| invalid_input("--collector-url requires --run-id"))?
        .clone();

    let client = CollectorClient::new(collector_url)?;
    Ok(client.run(&run_id).await?.trace)
}

fn print_text(diagnostic: &ProgressCollapseDiagnostic) {
    println!("[AgenticHound Diagnostic] Progress Collapse Detector");
    println!("run_id: {}", diagnostic.run_id);
    println!("diagnostic: {}", diagnostic.diagnostic);
    println!("severity: {}", severity_label(diagnostic.severity));
    println!("summary: {}", diagnostic.summary);
    println!("reasons:");
    for reason in &diagnostic.reasons {
        println!("- {reason}");
    }
    println!(
        "supporting_signals: spans={} retries={} errors={} duration_ms={} tokens={} cost_usd={:.4}",
        diagnostic.supporting_signals.span_count,
        diagnostic.supporting_signals.retry_span_count,
        diagnostic.supporting_signals.error_count,
        diagnostic.supporting_signals.duration_ms,
        diagnostic.supporting_signals.total_tokens,
        diagnostic.supporting_signals.total_cost_usd
    );
}

fn print_help() {
    println!("Usage:");
    println!("  agentichound diagnose --input <path> [--json]");
    println!("  agentichound diagnose --collector-url <url> --run-id <id> [--json]");
}

fn print_diagnose_help() {
    println!("Usage:");
    println!("  agentichound diagnose --input <path> [--json]");
    println!("  agentichound diagnose --collector-url <url> --run-id <id> [--json]");
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Low => "low",
        DiagnosticSeverity::Medium => "medium",
        DiagnosticSeverity::High => "high",
        DiagnosticSeverity::Critical => "critical",
    }
}

fn invalid_input(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.to_string())
}
