use qash_consensus::params::consensus_params_hash;
use qash_consensus::encoding::ENCODING_VERSION;
use qash_consensus::derive_leaf_index;
use qash_model::{run, HaltReason};
use qash_address::encode as addr_encode;

fn main() {
    // --- 1. Protocol fingerprint ---
    let params_hash = consensus_params_hash();
    print!("QASH consensus params hash : ");
    for b in &params_hash { print!("{:02x}", b); }
    println!();
    println!("Encoding version           : {}", ENCODING_VERSION);
    println!();

    // --- 2. Derive leaf indices for 4 genesis validators ---
    println!("=== Genesis validator addresses ===");
    let seed = [0u8; 32];
    for id in 1u64..=4 {
        let leaf = derive_leaf_index(id, 0, &seed);
        let addr = addr_encode(&leaf);
        println!("  validator {:2}: {}", id, addr);
    }
    println!();

    // --- 3. Steady-state simulation: 6 idle epochs ---
    println!("=== Steady-state simulation (6 idle epochs, 4 validators) ===");
    let (mut state, inputs) = qash_model::scenario::steady_state(4, 6);
    let trace = run(&mut state, &inputs);
    for o in &trace {
        println!(
            "  epoch {:3}  root={:.8}  V={:>10}  δ={:>8}  halt={}",
            o.epoch,
            hex_prefix(&o.state_root),
            o.v_convergence.raw(),
            o.delta_window.raw(),
            if o.halt_triggered { "YES" } else { "no" },
        );
    }
    println!();

    // --- 4. Near-halt simulation ---
    println!("=== Near-halt simulation (spike after window fills) ===");
    let (mut state2, inputs2) = qash_model::scenario::near_halt(4);
    let trace2 = run(&mut state2, &inputs2);
    for o in &trace2 {
        println!(
            "  epoch {:3}  root={:.8}  V={:>10}  δ={:>8}  halt={}",
            o.epoch,
            hex_prefix(&o.state_root),
            o.v_convergence.raw(),
            o.delta_window.raw(),
            if o.halt_triggered { "YES" } else { "no" },
        );
    }
    let final_halt = trace2.last().map(|o| o.halt_reason).unwrap_or(HaltReason::None);
    println!("  → final halt_reason = {:?}", final_halt);
    println!();

    // --- 5. Protocol health summary ---
    println!("=== Protocol health ===");
    let steady_ok = trace.iter().all(|o| !o.halt_triggered);
    let halt_ok   = trace2.last().map(|o| o.halt_triggered).unwrap_or(false);
    println!("  Steady-state: {} (all epochs green)", if steady_ok { "PASS" } else { "FAIL" });
    println!("  Halt-trigger: {} (spike causes halt)", if halt_ok   { "PASS" } else { "FAIL" });

    if !steady_ok || !halt_ok {
        eprintln!("QASH simulation health check FAILED");
        std::process::exit(1);
    }
}

fn hex_prefix(b: &[u8; 32]) -> String {
    b[..4].iter().map(|x| format!("{:02x}", x)).collect()
}
