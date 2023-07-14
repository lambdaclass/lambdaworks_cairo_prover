use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use functions::execution::run_verifier_bench_with_security_level;

use lambdaworks_stark::starks::proof::options::SecurityLevel;

pub mod functions;

fn verifier_benches(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("VERIFIER");
    run_verifier_bench(
        &mut group,
        "fibonacci/500",
        &cairo0_proof_path("fibonacci_500.proof"),
        &cairo0_proof_path("fibonacci_500_sec.proof"),
    );
    run_verifier_bench(
        &mut group,
        "fibonacci/1000",
        &cairo0_proof_path("fibonacci_1000.proof"),
        &cairo0_proof_path("fibonacci_1000_sec.proof"),
    );
}

fn cairo0_proof_path(program_name: &str) -> String {
    const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");
    const PROGRAM_BASE_REL_PATH: &str = "/benches/proofs/";
    let program_base_path = CARGO_DIR.to_string() + PROGRAM_BASE_REL_PATH;
    program_base_path + program_name
}

fn run_verifier_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    proof_path: &str,
    sec_proof_path: &str,
) {
    run_verifier_bench_with_security_level(
        group,
        &format!("80_bits/{benchname}"),
        proof_path,
        SecurityLevel::Conjecturable80Bits,
    );
    run_verifier_bench_with_security_level(
        group,
        &format!("128_bits/{benchname}"),
        sec_proof_path,
        SecurityLevel::Conjecturable128Bits,
    );
}

criterion_group!(benches, verifier_benches);
criterion_main!(benches);
