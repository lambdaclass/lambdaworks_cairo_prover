#[cfg(feature = "giza")]
use criterion::{black_box, measurement::WallTime, BenchmarkGroup};
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "giza")]
use lambdaworks_stark::{
    cairo::{
        air::generate_cairo_proof,
        runner::run::{generate_prover_args, CairoVersion},
    },
    starks::proof::options::{self, SecurityLevel},
};
#[cfg(feature = "giza")]
pub mod functions;

#[cfg(not(feature = "giza"))]
fn cairo_benches(_: &mut Criterion) {
    println!("Enable giza feature to run giza benchmarks");
}

#[cfg(feature = "giza")]
fn cairo_benches(c: &mut Criterion) {
    #[cfg(feature = "parallel")]
    {
        let num_threads: usize = std::env::var("NUM_THREADS")
            .unwrap_or("8".to_string())
            .parse()
            .unwrap();
        println!("Running benchmarks using {} threads", num_threads);
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .unwrap();
    };

    let mut group = c.benchmark_group("CAIRO");
    group.sample_size(10);
    let proof_options = options::ProofOptions::new_secure(SecurityLevel::Provable80Bits, 3);

    run_lambdaworks_bench(
        &mut group,
        &proof_options,
        "lambdaworks/fibonacci/1000",
        &cairo0_program_path("fibonacci_1000.json"),
    );
    run_giza_bench(
        &mut group,
        &proof_options,
        "giza/fibonacci/1000",
        &cairo0_program_path("fibonacci_1000.json"),
        &cairo0_program_path("fibonacci_1000.trace"),
        &cairo0_program_path("fibonacci_1000.mem"),
    );

    run_lambdaworks_bench(
        &mut group,
        &proof_options,
        "lambdaworks/fibonacci/10000",
        &cairo0_program_path("fibonacci_10000.json"),
    );
    run_giza_bench(
        &mut group,
        &proof_options,
        "giza/fibonacci/10000",
        &cairo0_program_path("fibonacci_10000.json"),
        &cairo0_program_path("fibonacci_10000.trace"),
        &cairo0_program_path("fibonacci_10000.mem"),
    );
}

#[cfg(feature = "giza")]
fn cairo0_program_path(program_name: &str) -> String {
    const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");
    const PROGRAM_BASE_REL_PATH: &str = "/cairo_programs/cairo0/";
    let program_base_path = CARGO_DIR.to_string() + PROGRAM_BASE_REL_PATH;
    program_base_path + program_name
}

#[cfg(feature = "giza")]
fn run_lambdaworks_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    proof_options: &options::ProofOptions,
    benchname: &str,
    program_path: &str,
) {
    let program_content = std::fs::read(program_path).unwrap();
    let (main_trace, pub_inputs) =
        generate_prover_args(&program_content, &CairoVersion::V0, &None).unwrap();

    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            black_box(generate_cairo_proof(&main_trace, &pub_inputs, proof_options).unwrap())
        });
    });
}

#[cfg(feature = "giza")]
fn run_giza_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    proof_options: &options::ProofOptions,
    benchname: &str,
    program_path: &str,
    trace_path: &str,
    memory_path: &str,
) {
    let proof_options = giza_prover::ProofOptions::with_proof_options(
        Some(proof_options.fri_number_of_queries),
        Some(proof_options.blowup_factor as usize),
        Some(proof_options.grinding_factor as u32),
        None,
        None,
    );

    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            let trace = giza_runner::ExecutionTrace::from_file(
                program_path.into(),
                trace_path.into(),
                memory_path.into(),
                None,
            );

            black_box(giza_prover::prove_trace(trace, &proof_options)).unwrap()
        });
    });
}

criterion_group!(benches, cairo_benches);
criterion_main!(benches);
