// Domain B tool: compute the QASH genesis hash using the 7-level cascade.
//
// Input:  GENESIS_CONSTANTS.toml (read from current directory)
// Method: strip the genesis_hash and lock_algorithm value lines, then run h_cascade
// Output: "QASH-CASCADE-7:<128 lowercase hex chars>" to stdout
//
// Run: cargo run --bin compute-genesis-hash

fn main() {
    let raw = std::fs::read("GENESIS_CONSTANTS.toml")
        .expect("GENESIS_CONSTANTS.toml not found in current directory");

    // Canonical form: keep field names but strip their values for the two
    // mutable meta fields so the hash is stable across lock_algorithm updates.
    let text = std::str::from_utf8(&raw).expect("GENESIS_CONSTANTS.toml is not valid UTF-8");
    let canonical: Vec<u8> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("genesis_hash") {
                "genesis_hash = \"\""
            } else if trimmed.starts_with("lock_algorithm") {
                "lock_algorithm = \"\""
            } else {
                line
            }
        })
        .flat_map(|l| l.bytes().chain(std::iter::once(b'\n')))
        .collect();

    let hash = qash::crypto::cascade::h_cascade(&canonical);

    print!("QASH-CASCADE-7:");
    for b in &hash {
        print!("{:02x}", b);
    }
    println!();
}
