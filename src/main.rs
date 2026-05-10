fn main() {
    // Hosted binary entrypoint: uses the consensus core (no_std crate) via path dependency
    let data = b"genesis";
    let hash = qash_consensus::consensus_hash(data);
    println!("qash consensus hash: {:02x?}", hash);
}
