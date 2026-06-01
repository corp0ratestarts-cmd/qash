//! Domain B MVP demonstrator CLI.
//!
//! The demo commands exercise the local offline incident receipt commit flow
//! without changing Domain A consensus behavior.

use qash_pal::mvp_demo_profile;
use qash_pal::mvp_vault::{
    decode_public_commitments, verify_disclosure_bundle, MvpReceiptVault, PUBLIC_COMMITMENTS_HEADER,
};
use sha3::{Digest, Sha3_256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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
    VerifyDisclosure,
    Status,
    ExportEvidence,
    VerifyEvidence,
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
            "verify-disclosure" => Some(Self::VerifyDisclosure),
            "status" => Some(Self::Status),
            "export-evidence" => Some(Self::ExportEvidence),
            "verify-evidence" => Some(Self::VerifyEvidence),
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
    DisclosureRequired,
    CommitmentsRequired,
    OutputRequired,
    EvidenceRequired,
    InvalidEvidence(String),
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
            Self::DisclosureRequired => write!(f, "--disclosure FILE is required"),
            Self::CommitmentsRequired => write!(f, "--commitments FILE is required"),
            Self::OutputRequired => write!(f, "--out PATH is required"),
            Self::EvidenceRequired => write!(f, "--evidence DIR is required"),
            Self::InvalidEvidence(message) => write!(f, "invalid MVP evidence: {message}"),
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
    commitments: Option<PathBuf>,
    disclosure: Option<PathBuf>,
    evidence: Option<PathBuf>,
    out: Option<PathBuf>,
    report: Option<PathBuf>,
    epoch: u64,
    nonce: Option<[u8; 32]>,
    body: Vec<u8>,
    disclosure_key_commitment: Option<[u8; 32]>,
    receipt_id: Option<[u8; 32]>,
    json: bool,
}

impl Default for DemoOptions {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_DIR),
            peer_dir: None,
            import: None,
            import_files: Vec::new(),
            import_label: None,
            commitments: None,
            disclosure: None,
            evidence: None,
            out: None,
            report: None,
            epoch: 0,
            nonce: None,
            body: DEFAULT_BODY.as_bytes().to_vec(),
            disclosure_key_commitment: None,
            receipt_id: None,
            json: false,
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
        DemoCommand::VerifyDisclosure => cmd_verify_disclosure(&options),
        DemoCommand::Status => cmd_status(&options),
        DemoCommand::ExportEvidence => cmd_export_evidence(&options),
        DemoCommand::VerifyEvidence => cmd_verify_evidence(&options),
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
    println!("  qash-demo verify-disclosure --disclosure FILE --commitments FILE");
    println!("  qash-demo status [--dir PATH] [--json]");
    println!("  qash-demo export-evidence --dir PATH --out DIR");
    println!("  qash-demo verify-evidence --evidence DIR");
    println!(
        "  qash-demo import-commitments --file FILE [--file FILE ...] [--label LABEL] [--dir PATH]"
    );
    println!("  qash-demo list-imports [--dir PATH]");
    println!();
    println!("Claim boundary:");
    println!("  This demo is not a payment instrument, settlement rail, credential");
    println!("  system, production attestation path, production ZK verifier, or");
    println!("  genesis-admitted transaction type.");
}

fn cmd_init(options: &DemoOptions) -> Result<(), DemoCliError> {
    MvpReceiptVault::init(&options.dir).map_err(vault_error)?;
    println!(
        "initialized QASH MVP demo workspace: {}",
        options.dir.display()
    );
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
        .issue_receipt(
            options.epoch,
            nonce,
            &options.body,
            disclosure_key_commitment,
        )
        .map_err(vault_error)?;
    println!("issued TX-MVP-ReceiptCommit");
    println!("workspace: {}", options.dir.display());
    println!("epoch: {}", receipt.tx.epoch);
    println!("receipt_id: {}", hex32(receipt.receipt_id));
    println!(
        "payload_commitment: {}",
        hex32(receipt.tx.payload_commitment)
    );
    println!("public export only contains commitments; private body remains in local vault");
    Ok(())
}

fn cmd_sync(options: &DemoOptions) -> Result<(), DemoCliError> {
    if let Some(import_path) = &options.import {
        let data = fs::read(import_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
        let vault = MvpReceiptVault::open(&options.dir)
            .or_else(|_| MvpReceiptVault::init(&options.dir))
            .map_err(vault_error)?;
        let count = vault
            .import_public_commitments(&data)
            .map_err(vault_error)?;
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
        println!(
            "synced commitment-only export to peer workspace: {}",
            peer_dir.display()
        );
    }
    println!(
        "wrote commitment-only public export: {}",
        out_path.display()
    );
    println!("bytes: {}", public.len());
    Ok(())
}

fn cmd_replay(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let (record_count, root) = replay_summary(&vault)?;
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
        fs::write(report_path, report.as_bytes())
            .map_err(|err| DemoCliError::Io(err.to_string()))?;
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

fn cmd_verify_disclosure(options: &DemoOptions) -> Result<(), DemoCliError> {
    let disclosure_path = options
        .disclosure
        .as_ref()
        .ok_or(DemoCliError::DisclosureRequired)?;
    let commitments_path = options
        .commitments
        .as_ref()
        .ok_or(DemoCliError::CommitmentsRequired)?;
    let disclosure = fs::read(disclosure_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    let commitments =
        fs::read(commitments_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    let bundle = verify_disclosure_bundle(&disclosure, &commitments).map_err(vault_error)?;
    println!("disclosure verification: ok");
    println!("receipt_id: {}", hex32(bundle.receipt_id));
    println!("body_bytes: {}", bundle.body.len());
    println!("commitments: {}", commitments_path.display());
    Ok(())
}

fn cmd_status(options: &DemoOptions) -> Result<(), DemoCliError> {
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    let local_records = vault.read_commitments().map_err(vault_error)?.len();
    let imports = vault.read_import_manifest().map_err(vault_error)?;
    let import_sources = imports.len();
    let imported_records = imports.iter().map(|entry| entry.records).sum::<usize>();
    let (records, root) = replay_summary(&vault)?;
    if options.json {
        println!(
            "{{\n  \"workspace\": \"{}\",\n  \"valid\": true,\n  \"local_records\": {},\n  \"import_sources\": {},\n  \"imported_records\": {},\n  \"replay_records\": {},\n  \"commitment_root\": \"{}\"\n}}",
            json_escape(&options.dir.display().to_string()),
            local_records,
            import_sources,
            imported_records,
            records,
            hex32(root)
        );
    } else {
        println!("QASH MVP workspace status");
        println!("workspace: {}", options.dir.display());
        println!("valid: true");
        println!("local_records: {local_records}");
        println!("import_sources: {import_sources}");
        println!("imported_records: {imported_records}");
        println!("replay_records: {records}");
        println!("commitment_root: {}", hex32(root));
    }
    Ok(())
}

fn cmd_export_evidence(options: &DemoOptions) -> Result<(), DemoCliError> {
    let out_dir = options.out.as_ref().ok_or(DemoCliError::OutputRequired)?;
    let vault = MvpReceiptVault::open(&options.dir).map_err(vault_error)?;
    fs::create_dir_all(out_dir).map_err(|err| DemoCliError::Io(err.to_string()))?;

    let public = export_all_public_commitments(&vault)?;
    let (records, root) = replay_summary(&vault)?;
    let imports = vault.read_import_manifest().map_err(vault_error)?;
    let status = status_json(&options.dir, records, root, &imports, &vault)?;
    let replay = replay_json(records, root);
    let imports_json = imports_json(&imports);

    let public_path = out_dir.join("public_commitments.bin");
    let replay_path = out_dir.join("replay.json");
    let status_path = out_dir.join("status.json");
    let imports_path = out_dir.join("imports.json");
    fs::write(&public_path, &public).map_err(|err| DemoCliError::Io(err.to_string()))?;
    fs::write(&replay_path, replay.as_bytes()).map_err(|err| DemoCliError::Io(err.to_string()))?;
    fs::write(&status_path, status.as_bytes()).map_err(|err| DemoCliError::Io(err.to_string()))?;
    fs::write(&imports_path, imports_json.as_bytes())
        .map_err(|err| DemoCliError::Io(err.to_string()))?;

    let manifest = evidence_manifest(&[
        ("public_commitments.bin", &public_path),
        ("replay.json", &replay_path),
        ("status.json", &status_path),
        ("imports.json", &imports_path),
    ])?;
    fs::write(out_dir.join("manifest.txt"), manifest.as_bytes())
        .map_err(|err| DemoCliError::Io(err.to_string()))?;

    println!("exported public MVP evidence: {}", out_dir.display());
    println!("records: {records}");
    println!("commitment_root: {}", hex32(root));
    println!("note: disclosure bundles and private receipt bodies are not exported");
    Ok(())
}

fn cmd_verify_evidence(options: &DemoOptions) -> Result<(), DemoCliError> {
    let evidence_dir = options
        .evidence
        .as_ref()
        .ok_or(DemoCliError::EvidenceRequired)?;
    let expected_files = [
        "public_commitments.bin",
        "replay.json",
        "status.json",
        "imports.json",
        "manifest.txt",
    ];
    for entry in fs::read_dir(evidence_dir).map_err(|err| DemoCliError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| DemoCliError::Io(err.to_string()))?;
        if !entry
            .file_type()
            .map_err(|err| DemoCliError::Io(err.to_string()))?
            .is_file()
        {
            return Err(DemoCliError::InvalidEvidence(format!(
                "unexpected non-file entry: {}",
                entry.path().display()
            )));
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            return Err(DemoCliError::InvalidEvidence(
                "non-UTF-8 evidence filename".to_string(),
            ));
        };
        if !expected_files.contains(&name.as_str()) {
            return Err(DemoCliError::InvalidEvidence(format!(
                "unexpected evidence file: {name}"
            )));
        }
    }

    let public_path = evidence_dir.join("public_commitments.bin");
    let replay_path = evidence_dir.join("replay.json");
    let status_path = evidence_dir.join("status.json");
    let imports_path = evidence_dir.join("imports.json");
    let manifest_path = evidence_dir.join("manifest.txt");

    let public = fs::read(&public_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    let exports = decode_public_commitments(&public).map_err(vault_error)?;
    let export_bytes: Vec<u8> = exports.iter().flat_map(|e| e.encode()).collect();
    let (records, root) = if export_bytes.is_empty() {
        (0, [0u8; 32])
    } else {
        let report = mvp_demo_profile::replay_public_export_bytes(&export_bytes)
            .map_err(|err| DemoCliError::Vault(format!("{err:?}")))?;
        (report.records, report.commitment_root)
    };

    verify_evidence_manifest(
        &manifest_path,
        &[
            ("public_commitments.bin", &public_path),
            ("replay.json", &replay_path),
            ("status.json", &status_path),
            ("imports.json", &imports_path),
        ],
    )?;

    let replay =
        fs::read_to_string(&replay_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    let expected_replay = replay_json(records, root);
    if replay != expected_replay {
        return Err(DemoCliError::InvalidEvidence(
            "replay report does not match public commitments".to_string(),
        ));
    }

    let status =
        fs::read_to_string(&status_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    require_json_field(&status, "\"valid\": true")?;
    require_json_field(&status, &format!("\"replay_records\": {records}"))?;
    require_json_field(
        &status,
        &format!("\"commitment_root\": \"{}\"", hex32(root)),
    )?;
    let _ = fs::read_to_string(&imports_path).map_err(|err| DemoCliError::Io(err.to_string()))?;

    println!("public MVP evidence verification: ok");
    println!("evidence: {}", evidence_dir.display());
    println!("records: {records}");
    println!("commitment_root: {}", hex32(root));
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
        println!(
            "  [{:04}] {} — {} records ({})",
            entry.seq, entry.label, entry.records, entry.file
        );
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
                options.peer_dir = Some(PathBuf::from(
                    required_value(args, i + 1, "--peer-dir")?.as_str(),
                ));
                i += 2;
            }
            "--import" => {
                options.import = Some(PathBuf::from(
                    required_value(args, i + 1, "--import")?.as_str(),
                ));
                i += 2;
            }
            "--file" => {
                options.import_files.push(PathBuf::from(
                    required_value(args, i + 1, "--file")?.as_str(),
                ));
                i += 2;
            }
            "--label" => {
                options.import_label = Some(required_value(args, i + 1, "--label")?.clone());
                i += 2;
            }
            "--commitments" => {
                options.commitments = Some(PathBuf::from(
                    required_value(args, i + 1, "--commitments")?.as_str(),
                ));
                i += 2;
            }
            "--disclosure" => {
                options.disclosure = Some(PathBuf::from(
                    required_value(args, i + 1, "--disclosure")?.as_str(),
                ));
                i += 2;
            }
            "--evidence" => {
                options.evidence = Some(PathBuf::from(
                    required_value(args, i + 1, "--evidence")?.as_str(),
                ));
                i += 2;
            }
            "--out" => {
                options.out = Some(PathBuf::from(
                    required_value(args, i + 1, "--out")?.as_str(),
                ));
                i += 2;
            }
            "--report" => {
                options.report = Some(PathBuf::from(
                    required_value(args, i + 1, "--report")?.as_str(),
                ));
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
                options.nonce = Some(parse_hex32(
                    required_value(args, i + 1, "--nonce-hex")?,
                    "--nonce-hex",
                )?);
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
                options.receipt_id = Some(parse_hex32(
                    required_value(args, i + 1, "--receipt-id")?,
                    "--receipt-id",
                )?);
                i += 2;
            }
            "--json" => {
                options.json = true;
                i += 1;
            }
            other => return Err(DemoCliError::UnknownFlag(other.to_string())),
        }
    }
    Ok(options)
}

fn required_value<'a>(
    args: &'a [String],
    index: usize,
    flag: &'static str,
) -> Result<&'a String, DemoCliError> {
    args.get(index).ok_or(DemoCliError::MissingValue(flag))
}

fn random_bytes() -> Result<[u8; 32], DemoCliError> {
    let mut out = [0u8; 32];
    getrandom::fill(&mut out).map_err(|err| DemoCliError::Random(err.to_string()))?;
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

fn replay_summary(vault: &MvpReceiptVault) -> Result<(usize, [u8; 32]), DemoCliError> {
    let exports = vault.read_all_public_exports().map_err(vault_error)?;
    let export_bytes: Vec<u8> = exports.iter().flat_map(|e| e.encode()).collect();
    if export_bytes.is_empty() {
        Ok((0, [0u8; 32]))
    } else {
        let rpt = mvp_demo_profile::replay_public_export_bytes(&export_bytes)
            .map_err(|e| DemoCliError::Vault(format!("{e:?}")))?;
        Ok((rpt.records, rpt.commitment_root))
    }
}

fn export_all_public_commitments(vault: &MvpReceiptVault) -> Result<Vec<u8>, DemoCliError> {
    let exports = vault.read_all_public_exports().map_err(vault_error)?;
    let mut out = Vec::new();
    out.extend_from_slice(PUBLIC_COMMITMENTS_HEADER);
    for export in exports {
        out.extend_from_slice(&export.encode());
    }
    Ok(out)
}

fn replay_json(records: usize, root: [u8; 32]) -> String {
    format!(
        "{{\n  \"profile\": \"TX-MVP-ReceiptCommit\",\n  \"profile_version\": 1,\n  \"records\": {},\n  \"commitment_root\": \"{}\",\n  \"public_transcript_only\": true,\n  \"private_payloads_seen\": false,\n  \"status\": \"ok\"\n}}\n",
        records,
        hex32(root)
    )
}

fn status_json(
    dir: &Path,
    records: usize,
    root: [u8; 32],
    imports: &[qash_pal::mvp_vault::ImportManifestEntry],
    vault: &MvpReceiptVault,
) -> Result<String, DemoCliError> {
    let local_records = vault.read_commitments().map_err(vault_error)?.len();
    let imported_records = imports.iter().map(|entry| entry.records).sum::<usize>();
    Ok(format!(
        "{{\n  \"workspace\": \"{}\",\n  \"valid\": true,\n  \"local_records\": {},\n  \"import_sources\": {},\n  \"imported_records\": {},\n  \"replay_records\": {},\n  \"commitment_root\": \"{}\"\n}}\n",
        json_escape(&dir.display().to_string()),
        local_records,
        imports.len(),
        imported_records,
        records,
        hex32(root)
    ))
}

fn imports_json(imports: &[qash_pal::mvp_vault::ImportManifestEntry]) -> String {
    let mut out = String::from("[\n");
    for (idx, entry) in imports.iter().enumerate() {
        let comma = if idx + 1 < imports.len() { "," } else { "" };
        out.push_str(&format!(
            "  {{\"seq\":{},\"label\":\"{}\",\"file\":\"{}\",\"records\":{}}}{}\n",
            entry.seq,
            json_escape(&entry.label),
            json_escape(&entry.file),
            entry.records,
            comma
        ));
    }
    out.push_str("]\n");
    out
}

fn evidence_manifest(entries: &[(&str, &Path)]) -> Result<String, DemoCliError> {
    let mut out = String::from("QASH MVP public evidence manifest\n");
    out.push_str("scope=Domain B offline incident receipt commit demonstrator\n");
    out.push_str("private_payloads_included=false\n\n");
    for (name, path) in entries {
        let bytes = fs::read(path).map_err(|err| DemoCliError::Io(err.to_string()))?;
        out.push_str(&format!(
            "{} sha3-256={} bytes={}\n",
            name,
            hex32(sha3_256(&bytes)),
            bytes.len()
        ));
    }
    Ok(out)
}

fn verify_evidence_manifest(
    manifest_path: &Path,
    entries: &[(&str, &Path)],
) -> Result<(), DemoCliError> {
    let manifest =
        fs::read_to_string(manifest_path).map_err(|err| DemoCliError::Io(err.to_string()))?;
    if !manifest.starts_with("QASH MVP public evidence manifest\n") {
        return Err(DemoCliError::InvalidEvidence(
            "invalid manifest header".to_string(),
        ));
    }
    if !manifest.contains("private_payloads_included=false\n") {
        return Err(DemoCliError::InvalidEvidence(
            "manifest does not exclude private payloads".to_string(),
        ));
    }
    for (name, path) in entries {
        let bytes = fs::read(path).map_err(|err| DemoCliError::Io(err.to_string()))?;
        let expected = format!(
            "{} sha3-256={} bytes={}\n",
            name,
            hex32(sha3_256(&bytes)),
            bytes.len()
        );
        if !manifest.contains(&expected) {
            return Err(DemoCliError::InvalidEvidence(format!(
                "manifest entry mismatch for {name}"
            )));
        }
    }
    Ok(())
}

fn require_json_field(document: &str, field: &str) -> Result<(), DemoCliError> {
    if document.contains(field) {
        Ok(())
    } else {
        Err(DemoCliError::InvalidEvidence(format!(
            "missing or mismatched field: {field}"
        )))
    }
}

fn sha3_256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn json_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
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
        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]),
            Ok(())
        );
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
        assert_eq!(
            run_demo_cli(&["sync".into(), "--dir".into(), dir_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&["replay".into(), "--dir".into(), dir_s]),
            Ok(())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disclosure_verify_status_and_evidence_export_flow() {
        let dir = temp_workspace("operator-ready");
        let out = temp_workspace("operator-evidence");
        let dir_s = dir.to_string_lossy().to_string();
        let out_s = out.to_string_lossy().to_string();
        let nonce = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string();
        let public_path = dir.join("public_commitments.bin");
        let disclosure_path = dir.join("disclosure.bin");

        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "issue-receipt".into(),
                "--dir".into(),
                dir_s.clone(),
                "--epoch".into(),
                "7".into(),
                "--nonce-hex".into(),
                nonce,
                "--body".into(),
                "synthetic operator incident".into(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "sync".into(),
                "--dir".into(),
                dir_s.clone(),
                "--out".into(),
                public_path.to_string_lossy().to_string(),
            ]),
            Ok(())
        );

        let receipt_id = {
            let vault = qash_pal::mvp_vault::MvpReceiptVault::open(&dir).unwrap();
            vault.read_commitments().unwrap()[0]
                .public_export
                .tx_commitment
        };
        assert_eq!(
            run_demo_cli(&[
                "disclose".into(),
                "--dir".into(),
                dir_s.clone(),
                "--receipt-id".into(),
                super::hex32(receipt_id),
                "--out".into(),
                disclosure_path.to_string_lossy().to_string(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "verify-disclosure".into(),
                "--disclosure".into(),
                disclosure_path.to_string_lossy().to_string(),
                "--commitments".into(),
                public_path.to_string_lossy().to_string(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "status".into(),
                "--dir".into(),
                dir_s.clone(),
                "--json".into()
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "export-evidence".into(),
                "--dir".into(),
                dir_s,
                "--out".into(),
                out_s.clone(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&["verify-evidence".into(), "--evidence".into(), out_s.clone(),]),
            Ok(())
        );

        let public = fs::read(out.join("public_commitments.bin")).unwrap();
        assert!(!public
            .windows(b"synthetic operator incident".len())
            .any(|w| w == b"synthetic operator incident"));
        assert!(out.join("manifest.txt").exists());
        assert!(out.join("replay.json").exists());
        assert!(out.join("status.json").exists());
        assert!(!out.join("disclosure.bin").exists());

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn evidence_verification_rejects_tampered_public_commitments() {
        let dir = temp_workspace("operator-tamper");
        let out = temp_workspace("operator-tamper-evidence");
        let dir_s = dir.to_string_lossy().to_string();
        let out_s = out.to_string_lossy().to_string();

        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "issue-receipt".into(),
                "--dir".into(),
                dir_s.clone(),
                "--body".into(),
                "synthetic operator incident".into(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "export-evidence".into(),
                "--dir".into(),
                dir_s,
                "--out".into(),
                out_s.clone(),
            ]),
            Ok(())
        );

        let public_path = out.join("public_commitments.bin");
        let mut public = fs::read(&public_path).unwrap();
        let last = public.last_mut().unwrap();
        *last ^= 0x01;
        fs::write(&public_path, public).unwrap();

        assert!(matches!(
            run_demo_cli(&["verify-evidence".into(), "--evidence".into(), out_s]),
            Err(DemoCliError::InvalidEvidence(_)) | Err(DemoCliError::Vault(_))
        ));

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&out);
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
        assert!(matches!(
            parse_hex32("abcd", "field"),
            Err(DemoCliError::InvalidHex("field"))
        ));
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
        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), node_a_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "issue-receipt".into(),
                "--dir".into(),
                node_a_s.clone(),
                "--body".into(),
                "synthetic offline incident".into(),
            ]),
            Ok(())
        );
        let export_path = node_a.join("public_commitments.bin");
        assert_eq!(
            run_demo_cli(&[
                "sync".into(),
                "--dir".into(),
                node_a_s.clone(),
                "--out".into(),
                export_path.to_string_lossy().to_string(),
            ]),
            Ok(())
        );

        // node-b imports from node-a
        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), node_b_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&[
                "import-commitments".into(),
                "--dir".into(),
                node_b_s.clone(),
                "--file".into(),
                export_path.to_string_lossy().to_string(),
                "--label".into(),
                "node-a".into(),
            ]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&["list-imports".into(), "--dir".into(), node_b_s.clone()]),
            Ok(())
        );
        // replay should include imported records
        assert_eq!(
            run_demo_cli(&["replay".into(), "--dir".into(), node_b_s]),
            Ok(())
        );

        let _ = fs::remove_dir_all(&node_a);
        let _ = fs::remove_dir_all(&node_b);
    }

    #[test]
    fn list_imports_on_empty_workspace() {
        let dir = temp_workspace("list-empty");
        let dir_s = dir.to_string_lossy().to_string();
        assert_eq!(
            run_demo_cli(&["init".into(), "--dir".into(), dir_s.clone()]),
            Ok(())
        );
        assert_eq!(
            run_demo_cli(&["list-imports".into(), "--dir".into(), dir_s]),
            Ok(())
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
