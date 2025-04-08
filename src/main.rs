pub fn main() {
    let target_dir = "/tmp/jolt-guest-targets";
    let program = guest::compile_blake2s_hash(target_dir);

    let prover_preprocessing = guest::preprocess_prover_blake2s_hash(&program);
    let verifier_preprocessing = guest::preprocess_verifier_blake2s_hash(&program);

    let prove_blake2s_hash = guest::build_prover_blake2s_hash(program, prover_preprocessing);
    let verify_blake2s_hash = guest::build_verifier_blake2s_hash(verifier_preprocessing);

    let (output, proof) = prove_blake2s_hash(b"Hello, world!");
    let is_valid = verify_blake2s_hash(b"Hello, world!", output, proof);

    println!("output: {:?}", output);
    println!("valid: {}", is_valid);
}
