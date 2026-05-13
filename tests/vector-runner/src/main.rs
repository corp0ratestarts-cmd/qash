use qash_consensus::{
    advance_epoch, EpochInput, EpochState, FixedPoint, HaltReason, ValidatorMetrics, MAX_VALIDATORS,
};
use qash_consensus::encoding::compute_leaf_index;
use qash_consensus::lyapunov::ConvergenceWindow;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut vectors = None;
    let mut out = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--vectors" => vectors = args.next().map(PathBuf::from),
            "--out" => out = args.next().map(PathBuf::from),
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok((
        vectors.unwrap_or_else(|| PathBuf::from("tests/vectors/vectors.v1.json")),
        out.unwrap_or_else(|| PathBuf::from("out.native.json")),
    ))
}

fn genesis_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
    }
}

fn idle_input(n: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        update_count: n,
    }
}

fn run_vectors(vector_path: PathBuf, out_path: PathBuf) -> io::Result<()> {
    // The checked-in vector file is the human-readable manifest. This runner
    // computes the current executable outputs and CI compares them across ISAs.
    let manifest = fs::read_to_string(vector_path)?;

    let fp_mul = FixedPoint::from_raw(400_000)
        .checked_mul(FixedPoint::from_raw(350_000))
        .expect("fixed vector multiplication must not overflow");

    let leaf = compute_leaf_index(1, 2, &[0xab; 32]);

    let mut state = genesis_state();
    let input0 = idle_input(state.validator_count);
    let epoch0 = advance_epoch(&mut state, &input0).expect("epoch 0 noop must advance");
    let input1 = idle_input(state.validator_count);
    let epoch1 = advance_epoch(&mut state, &input1).expect("epoch 1 noop must advance");

    let epoch0_root = hex(&epoch0.state_root);
    let epoch1_root = hex(&epoch1.state_root);
    let leaf_hex = hex(&leaf);

    for required in [
        fp_mul.raw().to_string(),
        leaf_hex.clone(),
        epoch0_root.clone(),
        epoch1_root.clone(),
    ] {
        if !manifest.contains(&required) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("vector manifest does not contain expected value {required}"),
            ));
        }
    }

    let body = format!(
        concat!(
            "{{\n",
            "  \"runner_version\": 1,\n",
            "  \"fixed_point\": {{\n",
            "    \"case\": \"0.4_times_0.35\",\n",
            "    \"raw\": {}\n",
            "  }},\n",
            "  \"leaf_index\": {{\n",
            "    \"validator_id\": 1,\n",
            "    \"epoch\": 2,\n",
            "    \"epoch_seed_hex\": \"{}\",\n",
            "    \"leaf_index_hex\": \"{}\"\n",
            "  }},\n",
            "  \"epochs\": [\n",
            "    {{\n",
            "      \"epoch\": 1,\n",
            "      \"expected_state_root_hex\": \"{}\",\n",
            "      \"expected_v_convergence_raw\": {},\n",
            "      \"expected_phi_safety_raw\": {},\n",
            "      \"expected_halted\": false\n",
            "    }},\n",
            "    {{\n",
            "      \"epoch\": 2,\n",
            "      \"expected_state_root_hex\": \"{}\",\n",
            "      \"expected_v_convergence_raw\": {},\n",
            "      \"expected_phi_safety_raw\": {},\n",
            "      \"expected_halted\": false\n",
            "    }}\n",
            "  ]\n",
            "}}\n"
        ),
        fp_mul.raw(),
        hex(&[0xab; 32]),
        leaf_hex,
        epoch0_root,
        epoch0.lyapunov.v_convergence.raw(),
        epoch0.lyapunov.phi_safety.raw(),
        epoch1_root,
        epoch1.lyapunov.v_convergence.raw(),
        epoch1.lyapunov.phi_safety.raw(),
    );

    fs::write(out_path, body)
}

fn main() {
    let (vectors, out) = parse_args().unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(2);
    });

    if let Err(err) = run_vectors(vectors, out) {
        eprintln!("vector runner failed: {err}");
        std::process::exit(1);
    }
}
