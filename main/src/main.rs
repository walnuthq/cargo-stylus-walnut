// Copyright 2023-2024, Offchain Labs, Inc.
// For licensing, see https://github.com/OffchainLabs/cargo-stylus/blob/main/licenses/COPYRIGHT.md

// Enable unstable test feature for benchmarks when nightly is available
#![cfg_attr(feature = "nightly", feature(test))]

use alloy_primitives::{TxHash, B256};
use clap::{ArgGroup, Args, CommandFactory, Parser, Subcommand};
use constants::DEFAULT_ENDPOINT;
use ethers::abi::Bytes;
use ethers::types::{H160, U256, H256};
use ethers::providers::Middleware;
use eyre::{bail, eyre, Context, Result};
use std::{
    fmt,
    path::{Path, PathBuf},
};
use tokio::runtime::Builder;
use trace::Trace;
use util::{color::Color, sys};

use std::process::Stdio;

// Conditional import for Unix-specific `CommandExt`
#[cfg(unix)]
use std::{env, os::unix::process::CommandExt};

// Conditional import for Windows
#[cfg(windows)]
use std::env;

use serde::{Deserialize, Serialize};
use serde_json::json;

mod activate;
mod cache;
mod check;
mod constants;
mod deploy;
mod docker;
mod export_abi;
mod gen;
mod hostio;
mod macros;
mod new;
mod project;
mod trace;
mod util;
mod verify;
mod wallet;

#[derive(Parser, Debug)]
#[command(name = "walnutdbg")]
#[command(bin_name = "cargo walnutdbg")]
#[command(author = "Walnut")]
#[command(about = "Cargo subcommand for debugging Stylus projects", long_about = None)]
#[command(propagate_version = true)]
#[command(version)]
struct Opts {
    #[command(subcommand)]
    command: Apis,
}

#[derive(Parser, Debug, Clone)]
enum Apis {
    /// Create a new Stylus project.
    New {
        /// Project name.
        name: PathBuf,
        /// Create a minimal contract.
        #[arg(long)]
        minimal: bool,
    },
    /// Initializes a Stylus project in the current directory.
    Init {
        /// Create a minimal contract.
        #[arg(long)]
        minimal: bool,
    },
    /// Export a Solidity ABI.
    ExportAbi {
        /// The output file (defaults to stdout).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Write a JSON ABI instead using solc. Requires solc.
        #[arg(long)]
        json: bool,
        /// Rust crate's features list. Required to include feature specific abi.
        #[arg(long)]
        rust_features: Option<Vec<String>>,
    },
    /// Activate an already deployed contract.
    #[command(visible_alias = "a")]
    Activate(ActivateConfig),
    #[command(subcommand)]
    /// Cache a contract using the Stylus CacheManager for Arbitrum chains.
    Cache(Cache),
    /// Check a contract.
    #[command(visible_alias = "c")]
    Check(CheckConfig),
    /// Deploy a contract.
    #[command(visible_alias = "d")]
    Deploy(DeployConfig),
    /// Verify the deployment of a Stylus contract.
    #[command(visible_alias = "v")]
    Verify(VerifyConfig),
    /// Generate c code bindings for a Stylus contract.
    Cgen { input: PathBuf, out_dir: PathBuf },
    /// Replay a transaction in gdb.
    #[command(visible_alias = "r")]
    Replay(ReplayArgs),
    /// Trace a transaction in walnut-dbg, capturing user function calls.
    #[command(visible_alias = "uf")]
    Usertrace(UsertraceArgs),
    /// Trace a transaction.
    #[command(visible_alias = "t")]
    Trace(TraceArgs),
    /// Simulate a transaction.
    #[command(visible_alias = "s")]
    Simulate(SimulateArgs),
}

#[derive(Args, Clone, Debug)]
struct CommonConfig {
    /// Arbitrum RPC endpoint.
    #[arg(short, long, default_value = DEFAULT_ENDPOINT)]
    endpoint: String,
    /// Whether to print debug info.
    #[arg(long)]
    verbose: bool,
    /// The path to source files to include in the project hash, which
    /// is included in the contract deployment init code transaction
    /// to be used for verification of deployment integrity.
    /// If not provided, all .rs files and Cargo.toml and Cargo.lock files
    /// in project's directory tree are included.
    #[arg(long)]
    source_files_for_project_hash: Vec<String>,
    #[arg(long)]
    /// Optional max fee per gas in gwei units.
    max_fee_per_gas_gwei: Option<String>,
    /// Specifies the features to use when building the Stylus binary.
    #[arg(long)]
    features: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
enum Cache {
    /// Places a bid on a Stylus contract to cache it in the Arbitrum chain's wasm cache manager.
    #[command(visible_alias = "b")]
    Bid(CacheBidConfig),
    /// Checks the status of a Stylus contract in the Arbitrum chain's wasm cache manager.
    #[command(visible_alias = "s")]
    Status(CacheStatusConfig),
    /// Checks the status of a Stylus contract in the Arbitrum chain's wasm cache manager.
    #[command()]
    SuggestBid(CacheSuggestionsConfig),
}

#[derive(Args, Clone, Debug)]
pub struct CacheBidConfig {
    /// Arbitrum RPC endpoint.
    #[arg(short, long, default_value = DEFAULT_ENDPOINT)]
    endpoint: String,
    /// Whether to print debug info.
    #[arg(long)]
    verbose: bool,
    /// Wallet source to use.
    #[command(flatten)]
    auth: AuthOpts,
    /// Deployed and activated contract address to cache.
    address: H160,
    /// Bid, in wei, to place on the desired contract to cache. A value of 0 is a valid bid.
    bid: u64,
    #[arg(long)]
    /// Optional max fee per gas in gwei units.
    max_fee_per_gas_gwei: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct CacheStatusConfig {
    /// Arbitrum RPC endpoint.
    #[arg(short, long, default_value = DEFAULT_ENDPOINT)]
    endpoint: String,
    /// Stylus contract address to check status in the cache manager.
    #[arg(long)]
    address: Option<H160>,
}

#[derive(Args, Clone, Debug)]
pub struct CacheSuggestionsConfig {
    /// Arbitrum RPC endpoint.
    #[arg(short, long, default_value = DEFAULT_ENDPOINT)]
    endpoint: String,
    /// Stylus contract address to suggest a minimum bid for in the cache manager.
    address: H160,
}

#[derive(Args, Clone, Debug)]
pub struct ActivateConfig {
    #[command(flatten)]
    common_cfg: CommonConfig,
    #[command(flatten)]
    data_fee: DataFeeOpts,
    /// Wallet source to use.
    #[command(flatten)]
    auth: AuthOpts,
    /// Deployed Stylus contract address to activate.
    #[arg(long)]
    address: H160,
    /// Whether or not to just estimate gas without sending a tx.
    #[arg(long)]
    estimate_gas: bool,
}

#[derive(Args, Clone, Debug)]
pub struct CheckConfig {
    #[command(flatten)]
    common_cfg: CommonConfig,
    #[command(flatten)]
    data_fee: DataFeeOpts,
    /// The WASM to check (defaults to any found in the current directory).
    #[arg(long)]
    wasm_file: Option<PathBuf>,
    /// Where to deploy and activate the contract (defaults to a random address).
    #[arg(long)]
    contract_address: Option<H160>,
}

#[derive(Args, Clone, Debug)]
struct DeployConfig {
    #[command(flatten)]
    check_config: CheckConfig,
    /// Wallet source to use.
    #[command(flatten)]
    auth: AuthOpts,
    /// Only perform gas estimation.
    #[arg(long)]
    estimate_gas: bool,
    /// If specified, will not run the command in a reproducible docker container. Useful for local
    /// builds, but at the risk of not having a reproducible contract for verification purposes.
    #[arg(long)]
    no_verify: bool,
    /// Cargo stylus version when deploying reproducibly to downloads the corresponding cargo-stylus-base Docker image.
    /// If not set, uses the default version of the local cargo stylus binary.
    #[arg(long)]
    cargo_stylus_version: Option<String>,
    /// If set, do not activate the program after deploying it
    #[arg(long)]
    no_activate: bool,
    /// The address of the deployer contract that deploys, activates, and initializes the stylus constructor.
    #[arg(long, value_name = "DEPLOYER_ADDRESS")]
    experimental_deployer_address: Option<H160>,
    /// The salt passed to the stylus deployer.
    #[arg(long, default_value_t = B256::ZERO)]
    experimental_deployer_salt: B256,
    /// The constructor arguments.
    #[arg(
        long,
        num_args(0..),
        value_name = "ARGS",
        allow_hyphen_values = true,
    )]
    experimental_constructor_args: Vec<String>,
    /// The amount of Ether sent to the contract through the constructor.
    #[arg(long, value_parser = parse_ether, default_value = "0")]
    experimental_constructor_value: U256,
}

#[derive(Args, Clone, Debug)]
pub struct VerifyConfig {
    #[command(flatten)]
    common_cfg: CommonConfig,
    /// Hash of the deployment transaction.
    #[arg(long)]
    deployment_tx: String,
    #[arg(long)]
    /// If specified, will not run the command in a reproducible docker container. Useful for local
    /// builds, but at the risk of not having a reproducible contract for verification purposes.
    no_verify: bool,
    /// Cargo stylus version when deploying reproducibly to downloads the corresponding cargo-stylus-base Docker image.
    /// If not set, uses the default version of the local cargo stylus binary.
    #[arg(long)]
    cargo_stylus_version: Option<String>,
}

#[derive(Args, Clone, Debug)]
struct ReplayArgs {
    #[command(flatten)]
    trace: TraceArgs,
    /// Whether to use stable Rust. Note that nightly is needed to expand macros.
    #[arg(short, long)]
    stable_rust: bool,
    /// Any features that should be passed to cargo build.
    #[arg(short, long)]
    features: Option<Vec<String>>,
    /// Which specific package to build during replay, if any.
    #[arg(long)]
    package: Option<String>,
    /// Whether this process is the child of another.
    #[arg(short, long, hide(true))]
    child: bool,
}

#[derive(Args, Clone, Debug)]
struct UsertraceArgs {
    #[command(flatten)]
    trace: TraceArgs,
    /// Whether to use stable Rust. Note that nightly is needed to expand macros.
    #[arg(short, long)]
    stable_rust: bool,
    /// Any features that should be passed to cargo build.
    #[arg(short, long)]
    features: Option<Vec<String>>,
    /// Which specific package to build during replay, if any.
    #[arg(long)]
    package: Option<String>,
    /// Whether this process is the child of another.
    #[arg(short, long, hide(true))]
    child: bool,
}

#[derive(Args, Clone, Debug)]
struct TraceArgs {
    /// RPC endpoint.
    #[arg(short, long, default_value = "http://localhost:8547")]
    endpoint: String,
    /// Tx to replay.
    #[arg(short, long)]
    tx: TxHash,
    /// Project path.
    #[arg(short, long, default_value = ".")]
    project: PathBuf,
    /// If set, use the native tracer instead of the JavaScript one. Notice the native tracer might not be available in the node.
    #[arg(short, long, default_value_t = false)]
    use_native_tracer: bool,
    /// If set, capture calls from `stylus_sdk::` and external calls as well.
    #[arg(long, default_value_t = false)]
    verbose_usertrace: bool,
    /// Comma-separated list of other crates to trace. Example: --trace-external-usertrace="std,core,other_contract"
    #[arg(
        long,
        use_value_delimiter = true,
        value_delimiter = ','
    )]
    trace_external_usertrace: Vec<String>,
    /// If passed, do NOT redirect walnut-dbg's output to `/dev/null`.
    /// By default, we silence walnut-dbg to keep console output clean.
    #[arg(long, default_value_t = false)]
    enable_walnutdbg_output: bool,

    /// Solidity contract address for EVM-level callTracer.
    #[arg(
        long,
        value_name = "ADDRESS",
        value_parser = clap::value_parser!(H160)
    )]
    addr_solidity: Option<H160>,
}

#[derive(Args, Clone, Debug)]
pub struct SimulateArgs {
    /// RPC endpoint.
    #[arg(short, long, default_value = "http://localhost:8547")]
    endpoint: String,

    /// From address.
    #[arg(short, long)]
    from: Option<H160>,

    /// To address.
    #[arg(short, long)]
    to: Option<H160>,

    /// Gas limit.
    #[arg(long)]
    gas: Option<u64>,

    /// Gas price.
    #[arg(long)]
    gas_price: Option<U256>,

    /// Value to send with the transaction.
    #[arg(short, long)]
    value: Option<U256>,

    /// Data to send with the transaction, as a hex string (with or without '0x' prefix).
    #[arg(short, long)]
    data: Option<Bytes>,

    /// Project path.
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    /// If set, use the native tracer instead of the JavaScript one.
    #[arg(short, long, default_value_t = false)]
    use_native_tracer: bool,
}

#[derive(Clone, Debug, Args)]
struct DataFeeOpts {
    /// Percent to bump the estimated activation data fee by.
    #[arg(long, default_value = "20")]
    data_fee_bump_percent: u64,
}

#[derive(Clone, Debug, Args)]
#[clap(group(ArgGroup::new("key").required(true).args(&["private_key_path", "private_key", "keystore_path"])))]
struct AuthOpts {
    /// File path to a text file containing a hex-encoded private key.
    #[arg(long)]
    private_key_path: Option<PathBuf>,
    /// Private key as a hex string. Warning: this exposes your key to shell history.
    #[arg(long)]
    private_key: Option<String>,
    /// Path to an Ethereum wallet keystore file (e.g. clef).
    #[arg(long)]
    keystore_path: Option<String>,
    /// Keystore password file.
    #[arg(long)]
    keystore_password_path: Option<PathBuf>,
}

impl fmt::Display for CommonConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Convert the vector of source files to a comma-separated string
        let mut source_files: String = "".to_string();
        if !self.source_files_for_project_hash.is_empty() {
            source_files = format!(
                "--source-files-for-project-hash={}",
                self.source_files_for_project_hash.join(", ")
            );
        }
        write!(
            f,
            "--endpoint={} {} {} {}",
            self.endpoint,
            match self.verbose {
                true => "--verbose",
                false => "",
            },
            source_files,
            match &self.max_fee_per_gas_gwei {
                Some(fee) => format!("--max-fee-per-gas-gwei {}", fee),
                None => "".to_string(),
            }
        )
    }
}

pub trait GasFeeConfig {
    fn get_max_fee_per_gas_wei(&self) -> Result<Option<u128>>;
    fn get_fee_str(&self) -> &Option<String>;
}

fn convert_gwei_to_wei(fee_str: &str) -> Result<u128> {
    let gwei = match fee_str.parse::<f64>() {
        Ok(fee) if fee >= 0.0 => fee,
        Ok(_) => bail!("Max fee per gas must be non-negative"),
        Err(_) => bail!("Invalid max fee per gas value: {}", fee_str),
    };

    if !gwei.is_finite() {
        bail!("Invalid gwei value: must be finite");
    }

    let wei = gwei * 1e9;
    if !wei.is_finite() {
        bail!("Overflow occurred in floating point multiplication of --max-fee-per-gas-gwei converting");
    }

    if wei < 0.0 || wei >= u128::MAX as f64 {
        bail!("Result outside valid range for wei");
    }

    Ok(wei as u128)
}

impl GasFeeConfig for CommonConfig {
    fn get_fee_str(&self) -> &Option<String> {
        &self.max_fee_per_gas_gwei
    }

    fn get_max_fee_per_gas_wei(&self) -> Result<Option<u128>> {
        match self.get_fee_str() {
            Some(fee_str) => Ok(Some(convert_gwei_to_wei(fee_str)?)),
            None => Ok(None),
        }
    }
}

impl GasFeeConfig for CacheBidConfig {
    fn get_fee_str(&self) -> &Option<String> {
        &self.max_fee_per_gas_gwei
    }

    fn get_max_fee_per_gas_wei(&self) -> Result<Option<u128>> {
        match self.get_fee_str() {
            Some(fee_str) => Ok(Some(convert_gwei_to_wei(fee_str)?)),
            None => Ok(None),
        }
    }
}

impl fmt::Display for CheckConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.common_cfg,
            match &self.wasm_file {
                Some(path) => format!("--wasm-file={}", path.display()),
                None => "".to_string(),
            },
            match &self.contract_address {
                Some(addr) => format!("--contract-address={:?}", addr),
                None => "".to_string(),
            },
        )
    }
}

impl fmt::Display for DeployConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.check_config,
            self.auth,
            match self.estimate_gas {
                true => "--estimate-gas".to_string(),
                false => "".to_string(),
            },
            match self.no_verify {
                true => "--no-verify".to_string(),
                false => "".to_string(),
            }
        )
    }
}

impl fmt::Display for AuthOpts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            match &self.private_key_path {
                Some(path) => format!("--private-key-path={}", path.display()),
                None => "".to_string(),
            },
            match &self.private_key {
                Some(key) => format!("--private-key={}", key.clone()),
                None => "".to_string(),
            },
            match &self.keystore_path {
                Some(path) => format!("--keystore-path={}", path.clone()),
                None => "".to_string(),
            },
            match &self.keystore_password_path {
                Some(path) => format!("--keystore-password-path={}", path.display()),
                None => "".to_string(),
            }
        )
    }
}

impl fmt::Display for VerifyConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} --deployment-tx={} {}",
            self.common_cfg,
            self.deployment_tx,
            match self.no_verify {
                true => "--no-verify".to_string(),
                false => "".to_string(),
            }
        )
    }
}

// prints help message and exits
fn exit_with_help_msg() -> ! {
    Opts::command().print_help().unwrap();
    std::process::exit(0);
}

// prints version information and exits
fn exit_with_version() -> ! {
    println!("{}", Opts::command().render_version());
    std::process::exit(0);
}

fn main() -> Result<()> {
    // skip the starting arguments passed from the OS and/or cargo.
    let mut args =
        env::args().skip_while(|x| x == "cargo" || x == "walnutdbg" || x.contains("cargo-walnutdbg"));

    let Some(arg) = args.next() else {
        exit_with_help_msg();
    };

    // perform any builtins
    match arg.as_str() {
        "--help" | "-h" => exit_with_help_msg(),
        "--version" | "-V" => exit_with_version(),
        _ => {}
    };

    // see if custom extension exists and is not a deprecated extension
    let custom = format!("cargo-walnutdbg-{arg}");
    if sys::command_exists(&custom) && !is_deprecated_extension(&custom) {
        let mut command = sys::new_command(&custom);
        command.arg(arg).args(args);

        // Execute command conditionally based on the platform
        #[cfg(unix)]
        let err = command.exec(); // Unix-specific execution
        #[cfg(windows)]
        let err = command.status(); // Windows-specific execution
        bail!("failed to invoke {:?}: {:?}", custom.red(), err);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = Opts::parse_from(args);
    // use the current thread for replay.
    let mut runtime = match opts.command {
        Apis::Replay(_) => Builder::new_current_thread(),
        Apis::Usertrace(_) => Builder::new_current_thread(),
        _ => Builder::new_multi_thread(),
    };
    let runtime = runtime.enable_all().build()?;
    runtime.block_on(main_impl(opts))
}

// Checks if a cargo stylus extension is an old, deprecated extension which is no longer
// supported. These extensions are now incorporated as part of the `cargo-stylus` command itself and
// will be the preferred method of running them.
fn is_deprecated_extension(subcommand: &str) -> bool {
    matches!(
        subcommand,
        "cargo-stylus-check" | "cargo-stylus-cgen" | "cargo-stylus-replay"
    )
}

async fn main_impl(args: Opts) -> Result<()> {
    macro_rules! run {
        ($expr:expr, $($msg:expr),+) => {
            $expr.wrap_err_with(|| eyre!($($msg),+))?
        };
    }

    match args.command {
        Apis::New { name, minimal } => {
            run!(new::new(&name, minimal), "failed to open new project");
        }
        Apis::Init { minimal } => {
            run!(new::init(minimal), "failed to initialize project");
        }
        Apis::ExportAbi {
            json,
            output,
            rust_features,
        } => {
            run!(
                export_abi::export_abi(output, json, rust_features),
                "failed to export abi"
            );
        }
        Apis::Activate(config) => {
            run!(
                activate::activate_contract(&config).await,
                "stylus activate failed"
            );
        }
        Apis::Simulate(args) => {
            run!(simulate(args).await, "failed to simulate transaction");
        }
        Apis::Cgen { input, out_dir } => {
            run!(gen::c_gen(&input, &out_dir), "failed to generate c code");
        }
        Apis::Trace(args) => run!(trace(args).await, "failed to trace tx"),
        Apis::Replay(args) => run!(replay(args).await, "failed to replay tx"),
        Apis::Usertrace(args) => run!(usertrace(args).await, "failed to usertrace tx"),
        Apis::Cache(subcommand) => match subcommand {
            Cache::Bid(config) => {
                run!(
                    cache::place_bid(&config).await,
                    "stylus cache place bid failed"
                );
            }
            Cache::SuggestBid(config) => {
                run!(
                    cache::suggest_bid(&config).await,
                    "stylus cache suggest-bid failed"
                );
            }
            Cache::Status(config) => {
                run!(
                    cache::check_status(&config).await,
                    "stylus cache status failed"
                );
            }
        },
        Apis::Check(config) => {
            run!(check::check(&config).await, "stylus checks failed");
        }
        Apis::Deploy(config) => {
            if config.no_verify {
                run!(deploy::deploy(config).await, "stylus deploy failed");
            } else {
                println!(
                    "Running in a Docker container for reproducibility, this may take a while",
                );
                println!("NOTE: You can opt out by doing --no-verify");
                let mut commands: Vec<String> =
                    vec![String::from("deploy"), String::from("--no-verify")];
                let config_args = config
                    .to_string()
                    .split(' ')
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>();
                commands.extend(config_args);
                run!(
                    docker::run_reproducible(config.cargo_stylus_version, &commands),
                    "failed reproducible run"
                );
            }
        }
        Apis::Verify(config) => {
            if config.no_verify {
                run!(verify::verify(config).await, "failed to verify");
            } else {
                println!(
                    "Running in a Docker container for reproducibility, this may take a while",
                );
                let mut commands: Vec<String> =
                    vec![String::from("verify"), String::from("--no-verify")];
                let config_args = config
                    .to_string()
                    .split(' ')
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>();
                commands.extend(config_args);
                run!(
                    docker::run_reproducible(config.cargo_stylus_version, &commands),
                    "failed reproducible run"
                );
            }
        }
    }
    Ok(())
}

async fn trace(args: TraceArgs) -> Result<()> {
    let provider = sys::new_provider(&args.endpoint)?;
    let trace = Trace::new(provider, args.tx, args.use_native_tracer).await?;
    println!("{}", trace.json);
    Ok(())
}

async fn simulate(args: SimulateArgs) -> Result<()> {
    let provider = sys::new_provider(&args.endpoint)?;
    let trace = Trace::simulate(provider, &args).await?;
    println!("{}", trace.json);
    Ok(())
}

fn derive_crate_name(shared_library: &Path) -> String {
    let stem = shared_library
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let crate_name = stem.strip_prefix("lib").unwrap_or(&stem);
    crate_name.to_string()
}

#[derive(Debug, Deserialize, Serialize)]
struct DebugTraceResult {
    #[serde(rename = "calls")]
    calls: Vec<DebugTraceCall>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DebugTraceCall {
    from: H160,
    to: H160,
    input: String,
    #[serde(default)]
    calls: Vec<DebugTraceCall>,
}

async fn usertrace(args: UsertraceArgs) -> eyre::Result<()> {
    let macos = cfg!(target_os = "macos");
    build_shared_library(&args.trace.project, args.package, args.features)?;
    let library_extension = if macos { ".dylib" } else { ".so" };
    let shared_library = find_shared_library(&args.trace.project, library_extension)?;
    let crate_name = derive_crate_name(&shared_library);

    // 1) Get the receipt & print the to-address
    let provider = sys::new_provider(&args.trace.endpoint)?;
    let eth_tx_hash = H256::from_slice(args.trace.tx.as_ref());
    if let Some(receipt) = provider.get_transaction_receipt(eth_tx_hash).await? {
        if let Some(to_addr) = receipt.to {
            println!("Tracing contract at address: \x1b[1;32m{:?}\x1b[0m", to_addr);
        } else {
            eprintln!("Warning: tx {} has no “to” address", args.trace.tx);
        }
    } else {
        eprintln!("Warning: no receipt found for tx {}", args.trace.tx);
    }

    // 2) If the user supplied --addr-solidity, pull the EVM callTracer tree
    if let Some(sol_addr) = args.trace.addr_solidity {
        let raw = json!([
            args.trace.tx,
            { "tracer": "callTracer", "to": sol_addr }
        ]);
        let dbg: DebugTraceResult = provider
            .request::<serde_json::Value, DebugTraceResult>(
                "debug_traceTransaction",
                raw,
            )
            .await
            .wrap_err("debug_traceTransaction failed")?;

        let _ = std::fs::remove_file("/tmp/sol_trace.json");
        // write out the solidity tracer JSON for the pretty-printer
        std::fs::write(
            "/tmp/sol_trace.json",
            serde_json::to_string_pretty(&dbg)?,
        )?;

        println!("Fetched Solidity callTracer into /tmp/sol_trace.json\n");
    }

    // 3) Build the walnut-dbg calltrace command
    let mut crates_to_trace = vec![crate_name];
    if args.trace.verbose_usertrace {
        crates_to_trace.push("stylus_sdk".to_string());
    }
    crates_to_trace.extend(args.trace.trace_external_usertrace.clone());
    let pattern = format!("^({})::", crates_to_trace.join("|"));
    let calltrace_cmd = format!("calltrace start '{}'", pattern);

    // 4) Non-child: spawn walnut-dbg + pretty-print
    if !args.child {
        // remove any stale LLDB trace
        let _ = std::fs::remove_file("/tmp/lldb_function_trace.json");

        // invoke walnut-dbg
        let (cmd_name, cmd_args) = if sys::command_exists("rust-walnut-dbg") {
            ("rust-walnut-dbg", &[
                "-o", "b user_entrypoint",
                "-o", "r",
                "-o", &calltrace_cmd,
                "-o", "c",
                "-o", "calltrace stop",
                "-o", "q",
                "--",
            ][..])
        } else {
            bail!("rust-walnut-dbg not installed");
        };
        let mut dbg_cmd = sys::new_command(cmd_name);
        dbg_cmd.args(cmd_args);
        // forward all original args and append child flag
        for a in std::env::args() {
            dbg_cmd.arg(a);
        }
        dbg_cmd.arg("--child");
        if !args.trace.enable_walnutdbg_output {
            dbg_cmd.stdin(Stdio::null())
                   .stdout(Stdio::null())
                   .stderr(Stdio::null());
        }
        let status = dbg_cmd.status()?;
        if !status.success() {
            bail!("walnut-dbg returned {}", status);
        }

        // now pretty-print both trees
        let mut pp = sys::new_command("pretty-print-trace");
        pp.arg("/tmp/lldb_function_trace.json");
        // only pass solidity file if we generated it
        if args.trace.addr_solidity.is_some() {
            pp.arg("/tmp/sol_trace.json");
        }
        let mut child = pp.spawn()?;
        let _ = child.wait();

        return Ok(());
    }

    // 5) Child: actually replay the WASM
    let provider = sys::new_provider(&args.trace.endpoint)?;
    let trace = Trace::new(provider, args.trace.tx, args.trace.use_native_tracer).await?;
    let args_len = trace.tx.input.len();

    unsafe {
        *hostio::FRAME.lock() = Some(trace.reader());
        type Entrypoint = unsafe extern "C" fn(usize) -> usize;
        let lib = libloading::Library::new(shared_library)?;
        let main: libloading::Symbol<Entrypoint> = lib.get(b"user_entrypoint")?;
        match main(args_len) {
            0 => println!("call completed successfully"),
            1 => println!("call reverted"),
            x => println!("call exited with unknown status code: {}", x.red()),
        }
    }

    Ok(())
}

async fn replay(args: ReplayArgs) -> Result<()> {
    let macos = cfg!(target_os = "macos");
    if !args.child {
        let gdb_args = [
            "--quiet",
            "-ex=set breakpoint pending on",
            "-ex=b user_entrypoint",
            "-ex=r",
            "--args",
        ]
        .as_slice();
        let lldb_args = [
            "--source-quietly",
            "-o",
            "b user_entrypoint",
            "-o",
            "r",
            "--",
        ]
        .as_slice();
        let (cmd_name, args) = if sys::command_exists("rust-gdb") && !macos {
            ("rust-gdb", &gdb_args)
        } else if sys::command_exists("rust-lldb") {
            ("rust-lldb", &lldb_args)
        } else {
            println!("rust specific debugger not installed, falling back to generic debugger");
            if sys::command_exists("gdb") && !macos {
                ("gdb", &gdb_args)
            } else if sys::command_exists("lldb") {
                ("lldb", &lldb_args)
            } else {
                bail!("no debugger found")
            }
        };

        let mut cmd = sys::new_command(cmd_name);
        for arg in args.iter() {
            cmd.arg(arg);
        }

        for arg in std::env::args() {
            cmd.arg(arg);
        }
        cmd.arg("--child");

        #[cfg(unix)]
        let err = cmd.exec();
        #[cfg(windows)]
        let err = cmd.status();

        bail!("failed to exec {cmd_name} {:?}", err);
    }

    let provider = sys::new_provider(&args.trace.endpoint)?;
    let trace = Trace::new(provider, args.trace.tx, args.trace.use_native_tracer).await?;

    build_shared_library(&args.trace.project, args.package, args.features)?;
    let library_extension = if macos { ".dylib" } else { ".so" };
    let shared_library = find_shared_library(&args.trace.project, library_extension)?;

    // TODO: don't assume the contract is top-level
    let args_len = trace.tx.input.len();

    unsafe {
        *hostio::FRAME.lock() = Some(trace.reader());

        type Entrypoint = unsafe extern "C" fn(usize) -> usize;
        let lib = libloading::Library::new(shared_library)?;
        let main: libloading::Symbol<Entrypoint> = lib.get(b"user_entrypoint")?;

        match main(args_len) {
            0 => println!("call completed successfully"),
            1 => println!("call reverted"),
            x => println!("call exited with unknown status code: {}", x.red()),
        }
    }
    Ok(())
}

pub fn build_shared_library(
    path: &Path,
    package: Option<String>,
    features: Option<Vec<String>>,
) -> Result<()> {
    let mut cargo = sys::new_command("cargo");

    cargo.current_dir(path).arg("build");

    if let Some(f) = features {
        cargo.arg("--features").arg(f.join(","));
    }
    if let Some(p) = package {
        cargo.arg("--package").arg(p);
    }

    cargo
        .arg("--lib")
        .arg("--locked")
        .arg("--target")
        .arg(rustc_host::from_cli()?)
        .output()?;
    Ok(())
}

pub fn find_shared_library(project: &Path, extension: &str) -> Result<PathBuf> {
    let triple = rustc_host::from_cli()?;
    let so_dir = project.join(format!("target/{triple}/debug/"));
    let so_dir = std::fs::read_dir(&so_dir)
        .map_err(|e| eyre!("failed to open {}: {e}", so_dir.to_string_lossy()))?
        .filter_map(|r| r.ok())
        .map(|r| r.path())
        .filter(|r| r.is_file());

    let mut file: Option<PathBuf> = None;
    for entry in so_dir {
        let Some(ext) = entry.file_name() else {
            continue;
        };
        let ext = ext.to_string_lossy();

        if ext.contains(extension) {
            if let Some(other) = file {
                let other = other.file_name().unwrap().to_string_lossy();
                bail!("more than one .so found: {ext} and {other}",);
            }
            file = Some(entry);
        }
    }
    let Some(file) = file else {
        bail!("failed to find .so");
    };
    Ok(file)
}

fn parse_ether(s: &str) -> Result<U256> {
    Ok(ethers::utils::parse_ether(s)?)
}
