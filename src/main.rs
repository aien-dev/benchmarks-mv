mod harness;
mod models;
mod svg_chart;

use clap::{Parser, Subcommand};
use models::BenchmarkData;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "aien-benchmarks")]
#[command(about = "Sovereign Systems Benchmark Suite CLI & Measurement Harness")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = "data/benchmarks_latest.json")]
    input: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute live measurement sweep, generate benchmark dataset, and render SVG charts
    Measure {
        #[arg(short, long, default_value_t = 10)]
        concurrency: u32,

        #[arg(short, long, default_value_t = 500)]
        requests: u32,

        #[arg(short, long, default_value_t = 20)]
        warmup: u32,

        #[arg(long, default_value = "data/benchmarks_latest.json")]
        output_json: String,

        #[arg(long, default_value = "assets")]
        output_assets: String,

        #[arg(long)]
        skip_standalone: bool,

        #[arg(long)]
        skip_live: bool,

        /// Skip the streaming LLM endpoint
        #[arg(long)]
        skip_llm: bool,

        #[command(flatten)]
        llm: LlmArgs,
    },
    /// Measure one streaming LLM endpoint and write an aien-bench-record/1 JSON record plus raw samples
    LlmStream {
        #[command(flatten)]
        llm: LlmArgs,

        /// Directory for the record and its samples file (relative to the working directory)
        #[arg(long, default_value = "records")]
        out_dir: String,
    },
    /// Compare a baseline and a candidate endpoint in baseline, candidate, candidate, baseline
    /// order (greedy, serial); tok/s is quoted only when every reply equals the serial
    /// reference. Point both arms at one server to measure the box's own noise.
    Abba {
        /// Baseline endpoint and labels (the --llm-* flags)
        #[command(flatten)]
        llm: LlmArgs,
        /// Candidate port (default: the baseline's)
        #[arg(long)]
        candidate_port: Option<u16>,
        /// Candidate host (default: the baseline's)
        #[arg(long)]
        candidate_host: Option<String>,
        /// Candidate model id (default: the baseline's)
        #[arg(long)]
        candidate_model: Option<String>,
        /// Candidate engine label (default: the baseline's)
        #[arg(long)]
        candidate_engine: Option<String>,
        #[arg(long, default_value = "baseline")]
        baseline_label: String,
        #[arg(long, default_value = "candidate")]
        candidate_label: String,
        /// File with one prompt per non-empty line (default: a built-in set of three)
        #[arg(long)]
        prompts_file: Option<String>,
        /// ABBA rounds; each round is four cells
        #[arg(long, default_value_t = 1)]
        rounds: u32,
        /// Discarded passes over the prompts on the candidate before the first cell
        #[arg(long, default_value_t = 1)]
        candidate_warmup_passes: u32,
        /// Retries of one request while the server answers 503 busy
        #[arg(long, default_value_t = 40)]
        busy_retries: u32,
        /// Longest wait between busy retries, in seconds
        #[arg(long, default_value_t = 30)]
        busy_backoff_max_secs: u64,
        /// Start no new request after this many seconds
        #[arg(long, default_value_t = 840)]
        deadline_secs: u64,
        /// Engine commit of the baseline / candidate server (written to the receipt)
        #[arg(long)]
        baseline_engine_commit: Option<String>,
        #[arg(long)]
        candidate_engine_commit: Option<String>,
        /// Checkpoint revision served by the baseline / candidate
        #[arg(long)]
        baseline_checkpoint: Option<String>,
        #[arg(long)]
        candidate_checkpoint: Option<String>,
        /// PID of the baseline / candidate server: its binary hash and loaded libraries
        /// are read from /proc into the receipt
        #[arg(long)]
        baseline_pid: Option<u32>,
        #[arg(long)]
        candidate_pid: Option<u32>,
        /// Directory for the receipt
        #[arg(long, default_value = "records")]
        out_dir: String,
    },
    /// Render SVG charts to assets/ directory
    Generate {
        #[arg(short, long, default_value = "assets")]
        output: String,
    },
    /// Print terminal ASCII summary table
    Report,
    /// Verify all metrics against their budget invariants
    Verify,
    /// Synchronize benchmark claims in documentation files from benchmarks_latest.json
    SyncDocs {
        /// Target markdown documentation files to synchronize
        #[arg(short, long)]
        target: Vec<String>,

        /// Check mode: exit with non-zero code if documentation differs from data
        #[arg(long)]
        check: bool,
    },
}

/// Streaming LLM endpoint settings (shared by `measure` and `llm-stream`). Defaults are the
/// endpoint and model id the old hardcoded harness used; engine and precision default to
/// "unlabeled" so a record never claims hardware or numerics nobody stated.
#[derive(clap::Args, Clone, Debug)]
struct LlmArgs {
    /// Endpoint host
    #[arg(long, default_value = "127.0.0.1")]
    llm_host: String,
    /// Endpoint port
    #[arg(long, default_value_t = 18006)]
    llm_port: u16,
    /// Request path (OpenAI-compatible chat completions)
    #[arg(long, default_value = "/v1/chat/completions")]
    llm_path: String,
    /// Model id sent in the request and reported in the record
    #[arg(long, default_value = "atlas-lightning-omni")]
    llm_model: String,
    /// Engine label reported in the record (what serves the endpoint)
    #[arg(long, default_value = "unlabeled")]
    llm_engine: String,
    /// Precision label reported in the record (for example BF16); "unlabeled" when omitted
    #[arg(long)]
    llm_precision: Option<String>,
    /// Context length label reported in the record (tokens)
    #[arg(long)]
    llm_context_length: Option<u64>,
    /// max_tokens sent in each request
    #[arg(long, default_value_t = 20)]
    llm_max_tokens: u32,
    /// Prompt sent as the single user message
    #[arg(
        long,
        default_value = "Explain unified memory architecture in ten words."
    )]
    llm_prompt: String,
    /// Measured requests (N); p95 is reported only when at least 20 succeed
    #[arg(long, default_value_t = 1)]
    llm_requests: u32,
    /// Requests in flight at once
    #[arg(long, default_value_t = 1)]
    llm_concurrency: u32,
    /// Warmup requests run first and discarded
    #[arg(long, default_value_t = 1)]
    llm_warmup: u32,
    /// Do not send stream_options.include_usage
    #[arg(long)]
    llm_no_usage_request: bool,
    /// temperature sent in each request (omitted when not given). Output tokens/s is printed
    /// only with 0 (greedy) and when every reply equals the first warmup reply.
    #[arg(long)]
    llm_temperature: Option<f64>,
    /// TCP connect timeout in seconds
    #[arg(long, default_value_t = 5)]
    llm_connect_timeout_secs: u64,
    /// Longest wait for any single socket read, in seconds (a timeout fails the request)
    #[arg(long, default_value_t = 20)]
    llm_read_timeout_secs: u64,
}

impl LlmArgs {
    fn to_config(&self) -> harness::llm_stream::StreamConfig {
        harness::llm_stream::StreamConfig {
            host: self.llm_host.clone(),
            port: self.llm_port,
            path: self.llm_path.clone(),
            model: self.llm_model.clone(),
            engine: self.llm_engine.clone(),
            precision: self.llm_precision.clone(),
            context_length: self.llm_context_length,
            max_tokens: self.llm_max_tokens,
            prompt: self.llm_prompt.clone(),
            requests: self.llm_requests,
            concurrency: self.llm_concurrency,
            warmup: self.llm_warmup,
            request_usage: !self.llm_no_usage_request,
            temperature: self.llm_temperature,
            connect_timeout: std::time::Duration::from_secs(self.llm_connect_timeout_secs),
            read_timeout: std::time::Duration::from_secs(self.llm_read_timeout_secs),
        }
    }
}

fn load_data(path: &str) -> BenchmarkData {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read benchmark data file {}: {}", path, e));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("Failed to parse benchmark JSON: {}", e))
}

fn save_data(path: &str, data: &BenchmarkData) {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).expect("Failed to create parent directory for dataset");
    }
    let json = serde_json::to_string_pretty(data).expect("Failed to serialize benchmark data");
    fs::write(path, json).expect("Failed to write benchmark data");
    println!("  [OK] Saved benchmark dataset to {}", path);
}

fn render_all_svgs(data: &BenchmarkData, output_dir: &str) {
    let out_dir = Path::new(output_dir);
    fs::create_dir_all(out_dir).expect("Failed to create output directory");

    let mem_svg = svg_chart::generate_memory_chart(data);
    let mem_path = out_dir.join("memory_comparison.svg");
    fs::write(&mem_path, mem_svg).expect("Failed to write memory SVG");
    println!("  [OK] Rendered {}", mem_path.display());

    let lat_svg = svg_chart::generate_latency_chart(data);
    let lat_path = out_dir.join("latency_comparison.svg");
    fs::write(&lat_path, lat_svg).expect("Failed to write latency SVG");
    println!("  [OK] Rendered {}", lat_path.display());

    println!(
        "All SVG benchmark charts successfully updated in {}.",
        output_dir
    );
}

fn print_report(data: &BenchmarkData) {
    println!("================================================================================");
    println!("  AIEN Sovereign Systems Performance Benchmark Report");
    println!(
        "  Hardware: {} ({} {})",
        data.hardware.system, data.hardware.processor, data.hardware.architecture
    );
    println!(
        "  Unified Memory: {} GB | OS: {}",
        data.hardware.memory_unified_gb, data.hardware.os
    );
    if !data.environment.rustc_version.is_empty() {
        println!("  Compiler: {}", data.environment.rustc_version);
        println!(
            "  Kernel: {} | Page Size: {} KB",
            data.environment.kernel_version, data.environment.page_size_kb
        );
    }
    println!("  Timestamp: {}", data.timestamp);
    println!("================================================================================\n");

    println!("--- 1. Memory Resident Set Size (RSS) ---");
    println!(
        "{:<28} | {:<32} | {:>10}",
        "Service", "Architecture", "RSS (MB)"
    );
    println!("{:-<28}-|-{:-<32}-|-{:-<10}", "", "", "");
    for m in &data.memory_rss {
        println!(
            "{:<28} | {:<32} | {:>10.2}",
            m.service, m.architecture, m.rss_mb
        );
    }
    println!();

    println!("--- 2. HTTP Gateway Latency (p50 TTFB) & Concurrency Throughput ---");
    println!(
        "{:<32} | {:<24} | {:>10} | {:>10} | {:>10}",
        "Endpoint", "Engine", "Req/sec", "p50 (ms)", "p95 (ms)"
    );
    println!(
        "{:-<32}-|-{:-<24}-|-{:-<10}-|-{:-<10}-|-{:-<10}",
        "", "", "", "", ""
    );
    for l in &data.latency_concurrency {
        let label = format!("{} {}", l.service, l.endpoint);
        println!(
            "{:<32} | {:<24} | {:>10.1} | {:>10.2} | {:>10.2}",
            label, l.engine, l.requests_per_sec, l.p50_ms, l.p95_ms
        );
    }
    println!();

    println!("--- 3. Local Neural Inference & SIMD Vectors (Grace Blackwell) ---");
    println!(
        "{:<42} | {:<32} | {:>10} | {:>10}",
        "Workload", "Model / Kernel", "p50 (ms)", "p95 (ms)"
    );
    println!("{:-<42}-|-{:-<32}-|-{:-<10}-|-{:-<10}", "", "", "", "");
    for n in &data.neural_inference {
        let p50_str = if n.p50_ms < 0.01 {
            format!("{:.4}", n.p50_ms)
        } else {
            format!("{:.2}", n.p50_ms)
        };
        let p95_str = match n.p95_ms {
            None => "n/a".to_string(),
            Some(v) if v < 0.01 => format!("{:.4}", v),
            Some(v) => format!("{:.2}", v),
        };
        println!(
            "{:<42} | {:<32} | {:>10} | {:>10}",
            n.workload, n.model, p50_str, p95_str
        );
    }
    println!("================================================================================");
}

fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Report) {
        Commands::Measure {
            concurrency,
            requests,
            warmup,
            output_json,
            output_assets,
            skip_standalone,
            skip_live,
            skip_llm,
            llm,
        } => {
            let config = harness::MeasurementConfig {
                concurrency,
                requests,
                warmup,
                skip_standalone,
                skip_live,
                llm: (!skip_llm).then(|| llm.to_config()),
            };

            let data = match harness::run_measurement_suite(config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Measurement sweep error: {}", e);
                    std::process::exit(1);
                }
            };

            print_report(&data);
            save_data(&output_json, &data);
            render_all_svgs(&data, &output_assets);
        }
        Commands::LlmStream { llm, out_dir } => {
            use harness::llm_stream;
            let cfg = llm.to_config();
            // Fail before measuring if the run cannot be bound to a commit.
            let source = match llm_stream::source_identity() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("llm-stream: {}; refusing to write an unbound record", e);
                    std::process::exit(1);
                }
            };
            println!(
                "Streaming benchmark: {} (model {}, engine {}), {} request(s), concurrency {}, warmup {}",
                cfg.endpoint(),
                cfg.model,
                cfg.engine,
                cfg.requests,
                cfg.concurrency,
                cfg.warmup
            );
            let run = llm_stream::run_stream_benchmark(&cfg);
            let mut summary = llm_stream::summarize(&run.samples, run.wall);
            // The first warmup request ran serially before the measured ones: it is the reference.
            summary.token_check =
                llm_stream::token_check(cfg.temperature, run.warmup.first(), &run.samples);
            harness::inference_bench::print_stream_summary(&summary);
            match llm_stream::write_run(Path::new(&out_dir), &cfg, &run, &summary, &source) {
                Ok(path) => println!("record: {}", path.display()),
                Err(e) => {
                    eprintln!("llm-stream: {}", e);
                    std::process::exit(1);
                }
            }
            if summary.requests_ok == 0 {
                std::process::exit(2);
            }
        }
        Commands::Abba {
            llm,
            candidate_port,
            candidate_host,
            candidate_model,
            candidate_engine,
            baseline_label,
            candidate_label,
            prompts_file,
            rounds,
            candidate_warmup_passes,
            busy_retries,
            busy_backoff_max_secs,
            deadline_secs,
            baseline_engine_commit,
            candidate_engine_commit,
            baseline_checkpoint,
            candidate_checkpoint,
            baseline_pid,
            candidate_pid,
            out_dir,
        } => {
            use harness::{abba, llm_stream};
            let source = match llm_stream::source_identity() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("abba: {}; refusing to write an unbound receipt", e);
                    std::process::exit(1);
                }
            };
            let prompts: Vec<String> = match &prompts_file {
                Some(p) => match fs::read_to_string(p) {
                    Ok(t) => t
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(str::to_string)
                        .collect(),
                    Err(e) => {
                        eprintln!("abba: read {}: {}", p, e);
                        std::process::exit(1);
                    }
                },
                None => vec![
                    "Name three primary colors.".to_string(),
                    "Count from one to ten in words.".to_string(),
                    "What is the boiling point of water at sea level in Celsius?".to_string(),
                ],
            };
            if prompts.is_empty() || rounds == 0 {
                eprintln!("abba: need at least one prompt and one round");
                std::process::exit(1);
            }
            let baseline = llm.to_config();
            let candidate = llm_stream::StreamConfig {
                host: candidate_host.unwrap_or_else(|| baseline.host.clone()),
                port: candidate_port.unwrap_or(baseline.port),
                model: candidate_model.unwrap_or_else(|| baseline.model.clone()),
                engine: candidate_engine.unwrap_or_else(|| baseline.engine.clone()),
                ..baseline.clone()
            };
            let cfg = abba::AbbaConfig {
                baseline,
                candidate,
                prompts,
                rounds,
                candidate_warmup_passes,
                busy_retries,
                busy_backoff_max: std::time::Duration::from_secs(busy_backoff_max_secs),
                deadline: std::time::Duration::from_secs(deadline_secs),
            };
            println!(
                "ABBA: baseline {} ({}) vs candidate {} ({}), {} prompt(s), {} round(s), max_tokens {}, greedy, serial",
                cfg.baseline.endpoint(),
                baseline_label,
                cfg.candidate.endpoint(),
                candidate_label,
                cfg.prompts.len(),
                rounds,
                cfg.baseline.max_tokens
            );
            let run = abba::run_abba(&cfg);
            let base = abba::summarize_arm(&run, &cfg, abba::Arm::Baseline);
            let cand = abba::summarize_arm(&run, &cfg, abba::Arm::Candidate);
            for c in &run.cells {
                let busy: u32 = c.requests.iter().map(|m| m.busy_retries).sum();
                println!(
                    "  cell {} {:<9} median decode tok/s {} | requests {} | busy retries {}",
                    c.index,
                    c.arm.as_str(),
                    c.median_rate()
                        .map_or("-".to_string(), |v| format!("{:.3}", v)),
                    c.requests.len(),
                    busy
                );
            }
            println!("{}", abba::arm_line(&base));
            println!("{}", abba::arm_line(&cand));
            match abba::delta(&base, &cand) {
                Some(d) => println!("candidate vs baseline: {:+.2}%", d * 100.0),
                None => println!(
                    "candidate vs baseline: no figure (UNVERIFIED tokens or missing cells)"
                ),
            }
            if let Some(why) = &run.stopped {
                println!("stopped early: {}", why);
            }
            let ids = (
                abba::ArmIdentity {
                    engine_commit: baseline_engine_commit,
                    checkpoint_revision: baseline_checkpoint,
                    server_pid: baseline_pid,
                },
                abba::ArmIdentity {
                    engine_commit: candidate_engine_commit,
                    checkpoint_revision: candidate_checkpoint,
                    server_pid: candidate_pid,
                },
            );
            let receipt = abba::build_receipt(
                &cfg,
                &run,
                &source,
                (&ids.0, &ids.1),
                (&baseline_label, &candidate_label),
            );
            let dir = Path::new(&out_dir);
            let path = dir.join(format!(
                "abba-{}.json",
                chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
            ));
            let written = fs::create_dir_all(dir).and_then(|_| {
                fs::write(
                    &path,
                    serde_json::to_string_pretty(&receipt).unwrap_or_default() + "\n",
                )
            });
            match written {
                Ok(()) => println!("receipt: {}", path.display()),
                Err(e) => {
                    eprintln!("abba: write {}: {}", path.display(), e);
                    std::process::exit(1);
                }
            }
            if !(base.tokens_verified() && cand.tokens_verified()) {
                std::process::exit(2);
            }
        }
        Commands::Report => {
            let data = load_data(&cli.input);
            print_report(&data);
        }
        Commands::Generate { output } => {
            let data = load_data(&cli.input);
            render_all_svgs(&data, &output);
        }
        Commands::Verify => {
            let data = load_data(&cli.input);
            for m in &data.memory_rss {
                if m.service == "openclaw-rs" || m.service == "aegis-runtime" {
                    assert!(m.rss_mb < 10.0, "{} RSS exceeded 10MB budget", m.service);
                }
                if m.service == "cortex-rs" {
                    assert!(m.rss_mb < 25.0, "cortex-rs RSS exceeded 25MB budget");
                }
            }
            for l in &data.latency_concurrency {
                if l.engine.contains("Rust") {
                    assert!(l.p50_ms < 10.0, "Rust service latency exceeded 10ms p50");
                    assert!(
                        l.requests_per_sec > 1000.0,
                        "Rust service throughput below 1000 req/s"
                    );
                }
            }
            println!("All invariant checks passed successfully.");
        }
        Commands::SyncDocs { target, check } => {
            let data = load_data(&cli.input);
            let table = generate_markdown_table(&data);

            let targets = if target.is_empty() {
                vec![
                    "../README.md".to_string(),
                    "../docs/STATE_OF_AIEN.md".to_string(),
                ]
            } else {
                target.clone()
            };

            let mut mismatch = false;
            for t in &targets {
                let p = Path::new(t);
                if !p.exists() {
                    println!("Target file not found, skipping: {}", t);
                    continue;
                }
                let content = fs::read_to_string(p).expect("Failed to read target file");
                let updated = sync_markdown_content(&content, &table);

                if updated != content {
                    if check {
                        eprintln!("Benchmark table out of date in: {}", t);
                        mismatch = true;
                    } else {
                        fs::write(p, &updated).expect("Failed to write target file");
                        println!("Synchronized benchmark table into: {}", t);
                    }
                } else {
                    println!("Benchmark table already in sync: {}", t);
                }
            }

            if mismatch {
                std::process::exit(1);
            }
        }
    }
}

pub fn generate_markdown_table(data: &BenchmarkData) -> String {
    let mut out = String::new();
    out.push_str("<!-- AIEN:BENCHMARKS:START -->\n");
    out.push_str("<!-- Sourced automatically from benchmarks/data/benchmarks_latest.json (Measurement Suite v");
    out.push_str(&data.version);
    out.push_str(") -->\n");
    out.push_str("| Workload / Service | Architecture | Measurement ID | Resident Memory (RSS) | p50 Latency | Throughput |\n");
    out.push_str("| :--- | :--- | :--- | :---: | :---: | :---: |\n");

    for mem in &data.memory_rss {
        let lat = data
            .latency_concurrency
            .iter()
            .find(|l| l.service == mem.service);
        let id = mem.measurement_id.as_deref().unwrap_or("BENCH-UNKNOWN");
        let rss_str = format!("{:.2} MB", mem.rss_mb);
        let (lat_str, tps_str) = match lat {
            Some(l) => (
                format!("{:.2} ms", l.p50_ms),
                format!("{:.0} req/s", l.requests_per_sec),
            ),
            None => ("N/A".to_string(), "N/A".to_string()),
        };

        out.push_str(&format!(
            "| {} | {} | `{}` | {} | {} | {} |\n",
            mem.role, mem.architecture, id, rss_str, lat_str, tps_str
        ));
    }

    out.push_str("\n*Hardware Reference: ");
    out.push_str(&data.hardware.system);
    out.push_str(" (");
    out.push_str(&data.hardware.processor);
    out.push_str(", ");
    out.push_str(&data.hardware.memory_unified_gb.to_string());
    out.push_str(" GB unified memory). All metrics measured under concurrency C=");
    out.push_str(&data.environment.concurrency_tested.to_string());
    out.push_str(" over ");
    out.push_str(&data.environment.requests_per_endpoint.to_string());
    out.push_str(" requests per endpoint.*\n");
    out.push_str("<!-- AIEN:BENCHMARKS:END -->");
    out
}

pub fn sync_markdown_content(content: &str, replacement: &str) -> String {
    let start_marker = "<!-- AIEN:BENCHMARKS:START -->";
    let end_marker = "<!-- AIEN:BENCHMARKS:END -->";

    let start_idx = match content.find(start_marker) {
        Some(idx) => idx,
        None => return content.to_string(),
    };

    let end_idx = match content.find(end_marker) {
        Some(idx) => idx + end_marker.len(),
        None => return content.to_string(),
    };

    let mut out = String::with_capacity(content.len() + replacement.len());
    out.push_str(&content[..start_idx]);
    out.push_str(replacement);
    out.push_str(&content[end_idx..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_generation() {
        let raw = include_str!("../data/benchmarks_20260919.json");
        let data: BenchmarkData = serde_json::from_str(raw).expect("Valid JSON");
        let mem_svg = svg_chart::generate_memory_chart(&data);
        assert!(mem_svg.contains("<svg"), "Must contain SVG opening tag");
        assert!(mem_svg.contains("</svg>"), "Must contain SVG closing tag");
        assert!(!mem_svg.contains('\u{2014}'), "Must not contain em dash");
        assert!(!mem_svg.contains('\u{2013}'), "Must not contain en dash");
        assert!(
            !mem_svg.to_lowercase().contains("python"),
            "No outside comparison in memory chart"
        );

        let lat_svg = svg_chart::generate_latency_chart(&data);
        assert!(lat_svg.contains("<svg"), "Must contain SVG opening tag");
        assert!(lat_svg.contains("</svg>"), "Must contain SVG closing tag");
        assert!(!lat_svg.contains('\u{2014}'), "Must not contain em dash");
        assert!(!lat_svg.contains('\u{2013}'), "Must not contain en dash");
    }

    #[test]
    fn test_sync_markdown_table_generation() {
        let raw = include_str!("../data/benchmarks_latest.json");
        let data: BenchmarkData = serde_json::from_str(raw).expect("Valid JSON");
        let table = generate_markdown_table(&data);

        assert!(table.contains("<!-- AIEN:BENCHMARKS:START -->"));
        assert!(table.contains("<!-- AIEN:BENCHMARKS:END -->"));
        assert!(table.contains("BENCH-OPENCLAW-RSS-001"));
        assert!(
            !table.to_lowercase().contains("python"),
            "No outside comparison rows"
        );
        assert!(table.contains("BENCH-CORTEX-RSS-001"));
        assert!(!table.contains('\u{2014}'), "Must not contain em dash");
        assert!(!table.contains('\u{2013}'), "Must not contain en dash");

        let doc = "# Header\n<!-- AIEN:BENCHMARKS:START -->\nold table\n<!-- AIEN:BENCHMARKS:END -->\n# Footer";
        let synced = sync_markdown_content(doc, &table);
        assert!(synced.contains("# Header"));
        assert!(synced.contains("# Footer"));
        assert!(synced.contains("BENCH-OPENCLAW-RSS-001"));
        assert!(!synced.contains("old table"));
    }
}
