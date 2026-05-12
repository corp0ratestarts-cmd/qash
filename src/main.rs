fn main() {
    let params_hash = qash_consensus::params::consensus_params_hash();
    print!("QASH consensus params hash: ");
    for b in &params_hash {
        print!("{:02x}", b);
    }
    println!();
    println!("encoding version: {}", qash_consensus::encoding::ENCODING_VERSION);
}
