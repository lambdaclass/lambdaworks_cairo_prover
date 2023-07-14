use criterion::{black_box, measurement::WallTime, BenchmarkGroup};
use lambdaworks_math::{
    field::fields::fft_friendly::stark_252_prime_field::Stark252PrimeField,
    traits::{Deserializable, Serializable},
};
use lambdaworks_stark::{
    cairo::{
        air::{generate_cairo_proof, verify_cairo_proof, PublicInputs},
        cairo_layout::CairoLayout,
        execution_trace::build_main_trace,
        runner::run::{build_pub_inputs, generate_prover_args, run_program, CairoVersion},
    },
    starks::{
        proof::{
            options::{ProofOptions, SecurityLevel},
            stark::StarkProof,
        },
        trace::TraceTable,
    },
};

pub fn run_cairo_bench_with_security_level(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
    security_level: SecurityLevel,
) {
    let (proof_options, main_trace, pub_inputs) =
        generate_prover_args_with_options(program_path, security_level);

    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            black_box(generate_cairo_proof(&main_trace, &pub_inputs, &proof_options).unwrap())
        });
    });
}

pub fn run_cairo_bench_and_measure_proof(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
    security_level: SecurityLevel,
) {
    let (proof_options, main_trace, pub_inputs) =
        generate_prover_args_with_options(program_path, security_level);
    let proof_size = generate_cairo_proof(&main_trace, &pub_inputs, &proof_options)
        .unwrap()
        .serialize()
        .len();
    println!("Proof size: {} bytes", proof_size);
    println!("Trace length: {} rows", main_trace.n_rows());

    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            black_box(generate_cairo_proof(&main_trace, &pub_inputs, &proof_options).unwrap())
        });
    });
}

fn generate_prover_args_with_options(
    program_path: &str,
    security_level: SecurityLevel,
) -> (ProofOptions, TraceTable<Stark252PrimeField>, PublicInputs) {
    let program_content = std::fs::read(program_path).unwrap();
    let proof_options = ProofOptions::new_secure(security_level, 3);
    let (main_trace, pub_inputs) =
        generate_prover_args(&program_content, &CairoVersion::V0, &None).unwrap();
    (proof_options, main_trace, pub_inputs)
}

pub fn run_verifier_bench_with_security_level(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
    security_level: SecurityLevel,
) {
    let (proof, pub_inputs) = load_proof_and_pub_inputs(program_path);
    let proof_options = ProofOptions::new_secure(security_level, 3);
    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            black_box(assert!(verify_cairo_proof(
                &proof,
                &pub_inputs,
                &proof_options
            )))
        });
    });
}

fn load_proof_and_pub_inputs(input_path: &str) -> (StarkProof<Stark252PrimeField>, PublicInputs) {
    let program_content = std::fs::read(input_path).unwrap();
    let mut bytes = program_content.as_slice();
    let proof_len = usize::from_be_bytes(bytes[0..8].try_into().unwrap());
    bytes = &bytes[8..];
    let proof = StarkProof::<Stark252PrimeField>::deserialize(&bytes[0..proof_len]).unwrap();
    bytes = &bytes[proof_len..];

    let public_inputs = PublicInputs::deserialize(bytes).unwrap();

    (proof, public_inputs)
}

pub fn run_trace_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
) {
    let program_content = std::fs::read(program_path).unwrap();

    let (register_states, memory, program_size, range_check_builtin_range) = run_program(
        None,
        CairoLayout::Small,
        &program_content,
        &CairoVersion::V0,
    )
    .unwrap();

    let mut pub_inputs = build_pub_inputs(
        range_check_builtin_range,
        &None,
        &register_states,
        &memory,
        program_size,
    );

    group.bench_function(benchname, |bench| {
        bench.iter(|| black_box(build_main_trace(&register_states, &memory, &mut pub_inputs)));
    });
}
