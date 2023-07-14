use lambdaworks_math::field::fields::fft_friendly::stark_252_prime_field::Stark252PrimeField;
use lambdaworks_math::traits::{Deserializable, Serializable};
use lambdaworks_stark::cairo::air::{generate_cairo_proof, verify_cairo_proof, PublicInputs};
use lambdaworks_stark::cairo::runner::run::{generate_prover_args, CairoVersion};
use lambdaworks_stark::starks::proof::options::{ProofOptions, SecurityLevel};
use lambdaworks_stark::starks::proof::stark::StarkProof;
use std::env;
use std::time::Instant;

fn generate_proof(
    input_path: &String,
    proof_options: &ProofOptions,
) -> Option<(StarkProof<Stark252PrimeField>, PublicInputs)> {
    let timer = Instant::now();

    let cairo_version = if input_path.contains(".casm") {
        println!("Running casm on CairoVM and generating trace ...");
        CairoVersion::V1
    } else {
        println!("Running program on CairoVM and generating trace ...");
        CairoVersion::V0
    };

    let Ok(program_content) = std::fs::read(input_path) else {
        println!("Error opening {input_path} file");
        return None;
    };

    let Ok((main_trace, pub_inputs)) =
        generate_prover_args(&program_content, &cairo_version, &None) else {
            println!("Error generating prover args");
            return None;
        };

    println!("  Time spent: {:?}", timer.elapsed());
    println!("  Generated main trace with {} rows\n", main_trace.n_rows());

    let timer = Instant::now();
    println!("Making proof ...");
    let proof = match generate_cairo_proof(&main_trace, &pub_inputs, proof_options) {
        Ok(p) => p,
        Err(e) => {
            println!("Error generating proof: {:?}", e);
            return None;
        }
    };

    println!("Time spent in proving: {:?}", timer.elapsed());

    let proof_size = proof.serialize().len();
    println!("Proof size: {} bytes\n", proof_size);

    Some((proof, pub_inputs))
}

fn verify_proof(
    proof: StarkProof<Stark252PrimeField>,
    pub_inputs: PublicInputs,
    proof_options: &ProofOptions,
) -> bool {
    let timer = Instant::now();

    println!("Verifying ...");
    let proof_verified = verify_cairo_proof(&proof, &pub_inputs, proof_options);
    println!("Time spent in verifying: {:?} \n", timer.elapsed());

    if proof_verified {
        println!("Verification succeded");
    } else {
        println!("Verification failed");
    }

    proof_verified
}

fn get_proof_options(arg: Option<&String>) -> Option<ProofOptions> {
    let default_coset_offset = ProofOptions::default_test_options().coset_offset;

    match arg {
        Some(arg) => {
            if arg == "-s" {
                println!("Using secure proof options");
                Some(ProofOptions::new_secure(
                    SecurityLevel::Conjecturable128Bits,
                    default_coset_offset,
                ))
            } else {
                None
            }
        }
        None => Some(ProofOptions::new_secure(
            SecurityLevel::Conjecturable80Bits,
            default_coset_offset,
        )),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run <command> [arguments]");
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "prove" => {
            if args.len() < 4 {
                println!("Usage: cargo run prove <input_path> <output_path> [-s]");
                return;
            }

            let input_path = &args[2];
            let output_path = &args[3];
            let Some(proof_options) = get_proof_options(args.get(4)) else { return; };

            let Some((proof, pub_inputs)) = generate_proof(input_path, &proof_options) else {
                return;
            };

            let mut bytes = vec![];
            let proof_bytes = proof.serialize();
            bytes.extend(proof_bytes.len().to_be_bytes());
            bytes.extend(proof_bytes);
            bytes.extend(pub_inputs.serialize());

            let Ok(()) = std::fs::write(output_path, bytes) else {
                println!("Error writing proof to file: {output_path}");
                return;
            };
            println!("Proof written to {output_path}");
        }
        "verify" => {
            if args.len() < 3 {
                println!("Usage: cargo run verify <input_path> [-s]");
                return;
            }

            let input_path = &args[2];
            let Some(proof_options) = get_proof_options(args.get(3)) else { return; };

            let Ok(program_content) = std::fs::read(input_path) else {
                println!("Error opening {input_path} file");
                return;
            };
            let mut bytes = program_content.as_slice();
            if bytes.len() < 8 {
                println!("Error reading proof from file: {input_path}");
                return;
            }
            let proof_len = usize::from_be_bytes(bytes[0..8].try_into().unwrap());
            bytes = &bytes[8..];
            if bytes.len() < proof_len {
                println!("Error reading proof from file: {input_path}");
                return;
            }
            let Ok(proof) = StarkProof::<Stark252PrimeField>::deserialize(&bytes[0..proof_len]) else {
                println!("Error reading proof from file: {input_path}");
                return;
            };
            bytes = &bytes[proof_len..];

            let Ok(pub_inputs) = PublicInputs::deserialize(bytes) else {
                println!("Error reading proof from file: {input_path}");
                return;
            };

            verify_proof(proof, pub_inputs, &proof_options);
        }
        "prove_and_verify" => {
            if args.len() < 3 {
                println!("Usage: cargo run prove_and_verify <input_path> [-s]");
                return;
            }

            let input_path = &args[2];
            let Some(proof_options) = get_proof_options(args.get(3)) else { return; };

            let Some((proof, pub_inputs)) = generate_proof(input_path, &proof_options) else {
                return;
            };
            verify_proof(proof, pub_inputs, &proof_options);
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
}
