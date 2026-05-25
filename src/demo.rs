//! Domain B MVP demonstrator CLI.
//!
//! The demo commands exercise the local offline incident receipt commit flow
//! without changing Domain A consensus behavior.

use qash_pal::mvp_vault::MvpReceiptVault;
use sha3::{Digest, Sha3_256};
use std::fmt;
use std::fs;
use std::path::PathBuf;

const DEMO_SCOPE: &str = "Domain B-only offline incident receipt commit demonstrator";
const DEFAULT_DIR: &str = ".qash-mvp-demo";
const DEFAULT_BODY: &str = "synthetic offline incident receipt";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoCommand {
    Init,
    IssueReceipt,
    Sync,
    Replay,
    Disclose,
}

impl DemoCommand {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "init" => Some(Self::Init),
            "issue-receipt" => Some(Self::IssueReceipt),
            "sync" => Some(Self::Sync),
            "replay" => Some(Self::Replay),
            "disclose" => Some(Self::Disclose),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DemoCliError {
    MissingCommand,
    UnknownCommand(String),
    UnknownFlag(String),
    MissingValue(&'static str),
    InvalidHex(&'static str),
    InvalidEpoch(String),
    Io(String),
    Random(String),
    Vault(String),
    ReceiptRequired,
}

impl fmt::Display for DemoCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing demo command"),
            Self::UnknownCommand(command) => write!(f, "unknown demo command: {command}"),
            Self::UnknownFlag(flag) => write!(f, "unknown demo flag: {flag}"),
            Self::MissingValue(flag) => write!(f, "missing value for {flag}"),
            Self::InvalidHex(field) => write!(f, "invalid 32-byte hex value for {field}"),
            Self::InvalidEpoch(value) => write!(f, "invalid epoch value: {value}"),
            Self::Io(message) => write!(f, "I/O error: {message}"),
            Self::Random(message) => write!(f, "randomness error: {message}"),
            Self::Vault(message) => write!(f, "MVP vault error: {message}"),
            Self::ReceiptRequired => write!(f, "--receipt-id <64 hex chars> is required"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoOptions {
    dir: PathBuf,
    peer_dir: Option<PathBuf>,
    out: Option<PathBuf>,
    epoch: u64,
    nonce: Option<[u8; 32]>,
    body: Vec<u8>,
    disclosure_key_commitment: Option<[u8; 32]>,
    receipt_id: Option<[u8; 32]>,
}

impl Default for DemoOptions {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_DIR),
            peer_dir: None,
            out: None,
            epoch: 0,
            nonce: None,
            body: DEFAULT_BODY.as_bytes().to_vec(),
            disclosure_key_commitment: None,
            receipt_id: None,
        }
    }
}

pub fn run_demo_cli(args: &[String]) -> Result<(), DemoCliError> {
    let Some(command_arg) = args.first() else {
        print_demo_help();
        return Err(DemoCliError::MissingCommand);
    };

    if command_arg == "--help" || command_arg == "-h" || command_arg == "help" {
        print_demo_help();
        return Ok(());
    }

    let Some(command) = DemoCommand::parse(command_arg) else {
        print_demo_help();
        return Err(DemoCliError::UnknownCommand(command_arg.clone()));
    };

    let options = parse_options(&args[1..])?;
    match command {
        DemoCommand::Init => cmd_init(&options),
        DemoCommand::IssueReceipt => cmd_issue_receipt(&options),
        DemoCommand::Sync => cmd_sync(&options),
        DemoCommand::Replay => cmd_replay(&options),
        DemoCommand::Disclose => cmd_disclose(&options),
    }
}

pub fn print_demo_help() {
    println!("QASH MVP demo CLI");
    println!();
    println!("Scope: {DEMO_SCOPE}");
    println!("Use case: offline incident-log attestation");
    println!("Transaction: TX-MVP-ReceiptCommit (Domain B demonstrator only)");
    println!();
    println!("Usage:");
    println!("  qash-demo init [--dir PATH]");
    println!("  qash-demo issue-receipt [--dir PATH] [--epoch N] [--nonce-hex HEX] [--body TEXT]");
    println!("  qash-demo sync [--dir PATH] [--out PATH] [--peer-dir PATH]");
    println!("  qash-demo replay [--dir PATH]");
    println!("  qash-demo disclose --receipt-id HEX [--dir PATH] [--out PATH]");
    println!();
    println!("Claim boundary:");
    println!("  This demo is not a payment instrument, settlement rail, credential");
    println!("  system, production attestation path, production ZK verifier, or");
    println!("  genesis-admitted transaction type.");
}

fn cmd_init(options: &DemoOptions) -> Result<(), DemoCliError> {
    MvpReceiptVault::init(&options.dir).map_err(vault_error)?;
    println!("initialized QASH MVP demo workspace: {}", options.dir.display());
    println!("scope: {DEMO_SCOPE}");
    Ok(())
}

fn cmd_issue_receipt(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir)
        .or_else(|_| MvpReceiptVault::init(&options.dir))
        .map_err(vault_error)?;
    let nonce = match options.nonce {
        Some(nonce) => nonce,
        None => random_bytes()?,
    };
    let disclosure_key_commitment = match options.disclosure_key_commitment {
        Some(commitment) => commitment,
        None => random_bytes()?,
    };

    let receipt = vault
        .issue_receipt(options.epoch, nonce, &options.body, disclosure_key_commitment)
        .map_err(vault_error)?;
    println!("issued TX-MVP-ReceiptCommit");
    println!("workspace: {}", options.dir.display());
    println!("epoch: {}", receipt.tx.epoch);
    println!("receipt_id: {}", hex32(receipt.receipt_id));
    println!("payload_commitment: {}", hex32(receipt.tx.payload_commitment));
    println!("public export only contains commitments; private body remains in local vault");
    Ok(())
}

fn cmd_sync(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let public = vault.export_public_commitments().map_err(vault_error)?;
    let out_path = options
        .out
        .clone()
        .unwrap_or_else(|| options.dir.join("public_commitments.bin"));
    fs::write(&out_path, &public).map_err(|err| DemoCliError::Io(err.to_string()))?;
    if let Some(peer_dir) = &options.peer_dir {
        fs::create_dir_all(peer_dir).map_err(|err| DemoCliError::Io(err.to_string()))?;
        fs::write(peer_dir.join("public_commitments.bin"), &public)
            .map_err(|err| DemoCliError::Io(err.to_string()))?;
        println!("synced commitment-only export to peer workspace: {}", peer_dir.display());
    }
    println!("wrote commitment-only public export: {}", out_path.display());
    println!("bytes: {}", public.len());
    Ok(())
}

fn cmd_replay(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let records = vault.read_commitments().map_err(vault_error)?;
    let mut root = [0u8; 32];
    for record in &records {
        root = replay_root_step(root, &record.public_export.encode());
    }
    println!("QASH MVP replay report");
    println!("workspace: {}", options.dir.display());
    println!("records: {}", records.len());
    println!("commitment_root: {}", hex32(root));
    println!("status: deterministic local replay completed");
    Ok(())
}

fn cmd_disclose(options: &DemoOptions) -> Result<(), DemoCliError> {
    let receipt_id = options.receipt_id.ok_or(DemoCliError::ReceiptRequired)?;
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let disclosure = vault.disclose_receipt(receipt_id).map_err(vault_error)?;
    let out_path = options
        .out
        .clone()
        .unwrap_or_else(|| options.dir.join("disclosure.bin"));
    fs::write(&out_path, &disclosure).map_err(|err| DemoCliError::Io(err.to_string()))?;
    println!("wrote selected receipt disclosure: {}", out_path.display());
    println!("receipt_id: {}", hex32(receipt_id));
    println!("bytes: {}", disclosure.len());
    Ok(())
}

fn parse_options(args: &[String]) -> Result<DemoOptions, DemoCliError> {
    let mut options = DemoOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                options.dir = PathBuf::from(required_value(args, i, "--dir")?.as_str());
            }
            "--peer-dir" => {
                i += 1;
                options.peer_dir = Some(PathBuf::from(required_value(args, i, "--peer-dir")?.as_str()));
            }
            "--out" => {
                i += 1;
                options.out = Some(PathBuf::from(required_value(args, i, "--out")?.as_str()));
            }
            "--epoch" => {
                i += 1;
                let value = required_value(args, i, "--epoch")?;
                options.epoch = value
                    .parse::<u64>()
                    .map_err(|_| DemoCliError::InvalidEpoch(value.clone()))?;
            }
            "--nonce-hex" => {
                i += 1;
                options.nonce = Some(parse_hex32(required_value(args, i, "--nonce-hex")?, "--nonce-hex")?);
            }
            "--body" => {
                i += 1;
                options.body = required_value(args, i, "--body")?.as_bytes().to_vec();
            }
            "--disclosure-key-commitment-hex" => {
                i += 1;
                options.disclosure_key_commitment = Some(parse_hex32(
                    required_value(args, i, "--disclosure-key-commitment-hex")?,
                    "--disclosure-key-commitment-hex",
                )?);
            }
            "--receipt-id" => {
                i += 1;
                options.receipt_id = Some(parse_hex32(required_value(args, i, "--receipt-id")?, "--receipt-id")?);
            }
            other => return Err(DemoCliError::UnknownFlag(other.to_string())),
        }
        i += 1;
    }
    Ok(options)
}

fn required_value<'a>(args: &'a [String], index: usize, flag: &'static str) -> Result<&'a String, DemoCliError> {
    args.get(index).ok_or(DemoCliError::MissingValue(flag))
}

fn replay_root_step(previous: [u8; 32], public_record: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(b"QASH-MVP-REPLAY-ROOT\0");
    hasher.update(previous);
    hasher.update((public_record.len() as u64).to_le_bytes());
    hasher.update(public_record);
    hasher.finalize().into()
}

fn random_bytes() -> Result<[u8; 32], DemoCliError> {
    let mut out = [0u8; 32];
    getrandom::getrandom(&mut out).map_err(|err| DemoCliError::Random(err.to_string()))?;
    Ok(out)
}

fn parse_hex32(value: &str, field: &'static str) -> Result<[u8; 32], DemoCliError> {
    if value.len() != 64 {
        return Err(DemoCliError::InvalidHex(field));
    }
    let mut out = [0u8; 32];
    for (idx, byte) in out.iter_mut().enumerate() {
        let start = idx * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| DemoCliError::InvalidHex(field))?;
    }
    Ok(out)
}

fn hex32(bytes: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");
    }
    out
}

fn vault_error(err: qash_pal::mvp_vault::MvpVaultError) -> DemoCliError {
    DemoCliError::Vault(format!("{err:?}"))
}

#[cfg(test)]
mod tests {
    use super::{parse_hex32, run_demo_cli, DemoCliError};
    use std::fs;

    fn temp_workspace(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("qash-cli-flow-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn demo_help_is_successful() {
        let args = vec!["help".to_string()];
        assert_eq!(run_demo_cli(&args), Ok(()));
    }

    #[test]
    fn demo_local_flow_runs() {
        let dir = temp_workspace("basic");
        let dir_s = dir.to_string_lossy().to_string();
        assert_eq!(run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]), Ok(()));
        assert_eq!(
            run_demo_cli(&[
                "issue-receipt".into(),
                "--dir".into(),
                dir_s.clone(),
                "--body".into(),
                "synthetic door alarm".into(),
            ]),
            Ok(())
        );
        assert_eq!(run_demo_cli(&["sync".into(), "--dir".into(), dir_s.clone()]), Ok(()));
        assert_eq!(run_demo_cli(&["replay".into(), "--dir".into(), dir_s]), Ok(()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_command_is_rejected() {
        let args = vec!["publish-graph".to_string()];
        assert_eq!(
            run_demo_cli(&args),
            Err(DemoCliError::UnknownCommand("publish-graph".to_string()))
        );
    }

    #[test]
    fn hex32_parser_rejects_bad_values() {
        assert!(matches!(parse_hex32("abcd", "field"), Err(DemoCliError::InvalidHex("field"))));
    }
}
