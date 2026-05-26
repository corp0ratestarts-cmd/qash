//! Domain B MVP demonstrator CLI.
//!
//! The demo commands exercise the local offline incident receipt commit flow
//! without changing Domain A consensus behavior.

use qash_pal::mvp_demo_profile;
use qash_pal::mvp_vault::MvpReceiptVault;
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
    ImportCommitments,
    ListImports,
}

impl DemoCommand {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "init" => Some(Self::Init),
            "issue-receipt" => Some(Self::IssueReceipt),
            "sync" => Some(Self::Sync),
            "replay" => Some(Self::Replay),
            "disclose" => Some(Self::Disclose),
            "import-commitments" => Some(Self::ImportCommitments),
            "list-imports" => Some(Self::ListImports),
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
    import: Option<PathBuf>,
    import_files: Vec<PathBuf>,
    import_label: Option<String>,
    out: Option<PathBuf>,
    report: Option<PathBuf>,
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
            import: None,
            import_files: Vec::new(),
            import_label: None,
            out: None,
            report: None,
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
        DemoCommand::ImportCommitments => cmd_import_commitments(&options),
        DemoCommand::ListImports => cmd_list_imports(&options),
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
    println!("  qash-demo sync --import FILE [--dir PATH]");
    println!("  qash-demo replay [--dir PATH] [--report PATH]");
    println!("  qash-demo disclose --receipt-id HEX [--dir PATH] [--out PATH]");
    println!("  qash-demo import-commitments --file FILE [--file FILE ...] [--label LABEL] [--dir PATH]");
    println!("  qash-demo list-imports [--dir PATH]");
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
        None => vault.fresh_nonce(options.epoch).map_err(vault_error)?,
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
    if let Some(import_path) = &options.import {
        let data = fs::read(import_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
        let vault = MvpReceiptVault::open(&options.dir)
            .or_else(|_| MvpReceiptVault::init(&options.dir))
            .map_err(vault_error)?;
        let count = vault.import_public_commitments(&data).map_err(vault_error)?;
        println!("imported {} public commitment records", count);
        println!("workspace: {}", options.dir.display());
        println!("source: {}", import_path.display());
        println!("note: imported records support replay but not disclosure (no private body)");
        return Ok(());
    }
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
    let exports = vault.read_all_public_exports().map_err(vault_error)?;
    let export_bytes: Vec<u8> = exports.iter().flat_map(|e| e.encode()).collect();
    let (record_count, root) = if export_bytes.is_empty() {
        (0usize, [0u8; 32])
    } else {
        let rpt = mvp_demo_profile::replay_public_export_bytes(&export_bytes)
            .map_err(|e| DemoCliError::Vault(format!("{e:?}")))?;
        (rpt.records, rpt.commitment_root)
    };
    println!("QASH MVP replay report");
    println!("workspace: {}", options.dir.display());
    println!("records: {record_count}");
    println!("commitment_root: {}", hex32(root));
    println!("status: deterministic local replay completed");
    if let Some(report_path) = &options.report {
        let report = format!(
            "{{\n  \"profile\": \"TX-MVP-ReceiptCommit\",\n  \"profile_version\": 1,\n  \"records\": {},\n  \"commitment_root\": \"{}\",\n  \"public_transcript_only\": true,\n  \"private_payloads_seen\": false,\n  \"status\": \"ok\"\n}}\n",
            record_count,
            hex32(root)
        );
        fs::write(report_path, report.as_bytes()).map_err(|err| DemoCliError::Io(err.to_string()))?;
        println!("report: {}", report_path.display());
    }
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

fn cmd_import_commitments(options: &DemoOptions) -> Result<(), DemoCliError> {
    if options.import_files.is_empty() {
        return Err(DemoCliError::MissingValue("--file"));
    }
    let vault = MvpReceiptVault::open(&options.dir)
        .or_else(|_| MvpReceiptVault::init(&options.dir))
        .map_err(vault_error)?;
    for file_path in &options.import_files {
        let data = fs::read(file_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
        let default_label = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let label = options.import_label.as_deref().unwrap_or(&default_label);
        let result = vault.import_with_label(&data, label).map_err(vault_error)?;
        println!("imported: {} (seq {})", result.label, result.seq);
        println!("  records: {}", result.records);
        println!("  new: {}", result.new_records);
        println!("  duplicates: {}", result.duplicates);
    }
    println!("workspace: {}", options.dir.display());
    println!("note: imported records support replay but not disclosure (no private body)");
    Ok(())
}

fn cmd_list_imports(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let entries = vault.read_import_manifest().map_err(vault_error)?;
    if entries.is_empty() {
        println!("no imports found in workspace: {}", options.dir.display());
        return Ok(());
    }
    println!("imports in workspace: {}", options.dir.display());
    for entry in &entries {
        println!("  [{:04}] {} — {} records ({})", entry.seq, entry.label, entry.records, entry.file);
    }
    println!("total: {} import(s)", entries.len());
    Ok(())
}

fn parse_options(args: &[String]) -> Result<DemoOptions, DemoCliError> {
    let mut options = DemoOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                options.dir = PathBuf::from(required_value(args, i + 1, "--dir")?.as_str());
                i += 2;
            }
            "--peer-dir" => {
                options.peer_dir = Some(PathBuf::from(required_value(args, i + 1, "--peer-dir")?.as_str()));
                i += 2;
            }
            "--import" => {
                options.import = Some(PathBuf::from(required_value(args, i + 1, "--import")?.as_str()));
                i += 2;
            }
            "--file" => {
                options.import_files.push(PathBuf::from(required_value(args, i + 1, "--file")?.as_str()));
                i += 2;
            }
            "--label" => {
                options.import_label = Some(required_value(args, i + 1, "--label")?.clone());
                i += 2;
            }
            "--out" => {
                options.out = Some(PathBuf::from(required_value(args, i + 1, "--out")?.as_str()));
                i += 2;
            }
            "--report" => {
                options.report = Some(PathBuf::from(required_value(args, i + 1, "--report")?.as_str()));
                i += 2;
            }
            "--epoch" => {
                let value = required_value(args, i + 1, "--epoch")?;
                options.epoch = value
                    .parse::<u64>()
                    .map_err(|_| DemoCliError::InvalidEpoch(value.clone()))?;
                i += 2;
            }
            "--nonce-hex" => {
                options.nonce = Some(parse_hex32(required_value(args, i + 1, "--nonce-hex")?, "--nonce-hex")?);
                i += 2;
            }
            "--body" => {
                options.body = required_value(args, i + 1, "--body")?.as_bytes().to_vec();
                i += 2;
            }
            "--disclosure-key-commitment-hex" => {
                options.disclosure_key_commitment = Some(parse_hex32(
                    required_value(args, i + 1, "--disclosure-key-commitment-hex")?,
                    "--disclosure-key-commitment-hex",
                )?);
                i += 2;
            }
            "--receipt-id" => {
                options.receipt_id = Some(parse_hex32(required_value(args, i + 1, "--receipt-id")?, "--receipt-id")?);
                i += 2;
            }
            other => return Err(DemoCliError::UnknownFlag(other.to_string())),
        }
    }
    Ok(options)
}

fn required_value<'a>(args: &'a [String], index: usize, flag: &'static str) -> Result<&'a String, DemoCliError> {
    args.get(index).ok_or(DemoCliError::MissingValue(flag))
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
    if !value.chars().all(|c| c.is_ascii_hexdigit()) {
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

    #[test]
    fn import_commitments_missing_file_flag_is_error() {
        let dir = temp_workspace("import-no-file");
        let dir_s = dir.to_string_lossy().to_string();
        let _ = run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]);
        let result = run_demo_cli(&["import-commitments".into(), "--dir".into(), dir_s]);
        assert_eq!(result, Err(DemoCliError::MissingValue("--file")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_commitments_and_list_flow() {
        let node_a = temp_workspace("import-node-a");
        let node_b = temp_workspace("import-node-b");
        let node_a_s = node_a.to_string_lossy().to_string();
        let node_b_s = node_b.to_string_lossy().to_string();

        // node-a issues a receipt and exports
        assert_eq!(run_demo_cli(&["init".into(), "--dir".into(), node_a_s.clone()]), Ok(()));
        assert_eq!(
            run_demo_cli(&[
                "issue-receipt".into(),
                "--dir".into(), node_a_s.clone(),
                "--body".into(), "synthetic offline incident".into(),
            ]),
            Ok(())
        );
        let export_path = node_a.join("public_commitments.bin");
        assert_eq!(
            run_demo_cli(&[
                "sync".into(),
                "--dir".into(), node_a_s.clone(),
                "--out".into(), export_path.to_string_lossy().to_string(),
            ]),
            Ok(())
        );

        // node-b imports from node-a
        assert_eq!(run_demo_cli(&["init".into(), "--dir".into(), node_b_s.clone()]), Ok(()));
        assert_eq!(
            run_demo_cli(&[
                "import-commitments".into(),
                "--dir".into(), node_b_s.clone(),
                "--file".into(), export_path.to_string_lossy().to_string(),
                "--label".into(), "node-a".into(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&["list-imports".into(), "--dir".into(), node_b_s.clone()]),
            Ok(())
        );
        // replay should include imported records
        assert_eq!(run_demo_cli(&["replay".into(), "--dir".into(), node_b_s]), Ok(()));

        let _ = fs::remove_dir_all(&node_a);
        let _ = fs::remove_dir_all(&node_b);
    }

    #[test]
    fn list_imports_on_empty_workspace() {
        let dir = temp_workspace("list-empty");
        let dir_s = dir.to_string_lossy().to_string();
        assert_eq!(run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]), Ok(()));
        assert_eq!(run_demo_cli(&["list-imports".into(), "--dir".into(), dir_s]), Ok(()));
        let _ = fs::remove_dir_all(&dir);
    }
}
