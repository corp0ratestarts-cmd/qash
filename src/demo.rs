//! Domain B MVP demonstrator CLI skeleton.
//!
//! The demo commands are intentionally placeholders in this slice. They reserve
//! the command surface for the offline incident receipt commit MVP without
//! changing Domain A consensus behavior.

use std::fmt;

const DEMO_SCOPE: &str = "Domain B-only offline incident receipt commit demonstrator";

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

    fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::IssueReceipt => "issue-receipt",
            Self::Sync => "sync",
            Self::Replay => "replay",
            Self::Disclose => "disclose",
        }
    }

    fn next_slice(self) -> &'static str {
        match self {
            Self::Init => "MVP-3 local vault/WAL initialization",
            Self::IssueReceipt => "MVP-2 receipt commit type and MVP-3 local vault/WAL",
            Self::Sync => "MVP-4 commitment-only sync",
            Self::Replay => "MVP-4 deterministic replay report",
            Self::Disclose => "MVP-5 one-receipt selective disclosure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemoCliError {
    MissingCommand,
    UnknownCommand(String),
    Placeholder(DemoCommand),
}

impl fmt::Display for DemoCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "missing demo command"),
            Self::UnknownCommand(command) => write!(f, "unknown demo command: {command}"),
            Self::Placeholder(command) => write!(
                f,
                "qash demo {} is reserved but not implemented yet ({})",
                command.name(),
                command.next_slice()
            ),
        }
    }
}

pub fn run_demo_cli(args: &[String]) -> Result<(), DemoCliError> {
    let Some(command) = args.first() else {
        print_demo_help();
        return Err(DemoCliError::MissingCommand);
    };

    if command == "--help" || command == "-h" || command == "help" {
        print_demo_help();
        return Ok(());
    }

    let Some(command) = DemoCommand::parse(command) else {
        print_demo_help();
        return Err(DemoCliError::UnknownCommand(command.clone()));
    };

    print_placeholder(command);
    Err(DemoCliError::Placeholder(command))
}

pub fn print_demo_help() {
    println!("QASH MVP demo CLI");
    println!();
    println!("Scope: {DEMO_SCOPE}");
    println!("Use case: offline incident-log attestation");
    println!("Transaction: TX-MVP-ReceiptCommit (Domain B demonstrator only)");
    println!();
    println!("Usage:");
    println!("  qash demo init");
    println!("  qash demo issue-receipt");
    println!("  qash demo sync");
    println!("  qash demo replay");
    println!("  qash demo disclose");
    println!();
    println!("Claim boundary:");
    println!("  This demo is not a payment instrument, settlement rail, credential");
    println!("  system, production attestation path, production ZK verifier, or");
    println!("  genesis-admitted transaction type.");
}

fn print_placeholder(command: DemoCommand) {
    println!("QASH MVP demo command reserved: {}", command.name());
    println!("Scope: {DEMO_SCOPE}");
    println!("Next implementation slice: {}", command.next_slice());
    println!("Status: not yet implemented in MVP-1 CLI skeleton");
}

#[cfg(test)]
mod tests {
    use super::{run_demo_cli, DemoCliError};

    #[test]
    fn demo_help_is_successful() {
        let args = vec!["help".to_string()];
        assert_eq!(run_demo_cli(&args), Ok(()));
    }

    #[test]
    fn known_commands_are_reserved_placeholders() {
        for name in ["init", "issue-receipt", "sync", "replay", "disclose"] {
            let args = vec![name.to_string()];
            assert!(matches!(run_demo_cli(&args), Err(DemoCliError::Placeholder(_))));
        }
    }

    #[test]
    fn unknown_command_is_rejected() {
        let args = vec!["publish-graph".to_string()];
        assert_eq!(
            run_demo_cli(&args),
            Err(DemoCliError::UnknownCommand("publish-graph".to_string()))
        );
    }
}
