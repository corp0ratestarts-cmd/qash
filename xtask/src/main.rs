use std::{
    env,
    path::PathBuf,
    process::{Command, ExitCode},
};

fn repo_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .expect("xtask must live one level below repo root")
        .to_owned()
}

fn run_script(root: &PathBuf, script: &str, args: &[&str]) -> ExitCode {
    let path = root.join("scripts").join(script);
    let status = Command::new("bash")
        .arg(&path)
        .args(args)
        .current_dir(root)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {script}: {e}"));
    if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let root = repo_root();
    let mut args = env::args().skip(1);
    let subcommand = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    let rest_strs: Vec<&str> = rest.iter().map(String::as_str).collect();

    match subcommand.as_str() {
        "verify-genesis" => run_script(&root, "verify_genesis_hash.sh", &rest_strs),
        "capture-evidence" => run_script(&root, "capture_pre_genesis_evidence.sh", &rest_strs),
        "audit-boundary" => run_script(&root, "check_domain_boundary.sh", &rest_strs),
        "audit-panic-surface" => run_script(&root, "check_panic_surface.sh", &rest_strs),
        "proof-hashes" => run_script(&root, "hash_proof_objects.sh", &rest_strs),
        "file-inventory" => run_script(&root, "file_inventory.sh", &rest_strs),
        "check-proof-counts" => run_script(&root, "check_proof_count_consistency.sh", &rest_strs),
        "help" | "--help" | "-h" | "" => {
            println!("Usage: cargo xtask <subcommand>");
            println!();
            println!("Subcommands:");
            println!("  verify-genesis       Recompute and verify the genesis artifact digest");
            println!("  capture-evidence     Run the full pre-genesis evidence capture bundle");
            println!("  audit-boundary       Check Domain A/B cross-contamination tripwires");
            println!("  audit-panic-surface  Scan Domain A for panic/unwrap surface");
            println!("  proof-hashes         Hash compiled Coq proof objects (.vo files)");
            println!("  file-inventory       Generate the complete repo file inventory");
            println!("  check-proof-counts   Verify proof vector counts are consistent across docs");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unknown subcommand `{other}`");
            eprintln!("Run `cargo xtask help` for usage.");
            ExitCode::FAILURE
        }
    }
}
