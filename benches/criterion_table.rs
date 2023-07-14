use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use functions::{
    execution::{
        run_cairo_bench_and_measure_proof, run_trace_bench, run_verifier_bench_with_security_level,
    },
    path::{cairo0_program_path, cairo0_proof_path},
};
use lambdaworks_stark::starks::proof::options::SecurityLevel;

pub mod functions;

fn table_benches(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("TABLE");
    group.sample_size(10);
    run_table_bench(
        &mut group,
        "fibonacci/100",
        &cairo0_program_path("fibonacci_100.json"),
        &cairo0_proof_path("fibonacci_100.proof"),
        &cairo0_proof_path("fibonacci_100_sec.proof"),
    );
    run_table_bench(
        &mut group,
        "fibonacci/500",
        &cairo0_program_path("fibonacci_500.json"),
        &cairo0_proof_path("fibonacci_500.proof"),
        &cairo0_proof_path("fibonacci_500_sec.proof"),
    );
    run_table_bench(
        &mut group,
        "fibonacci/2000",
        &cairo0_program_path("fibonacci_2000.json"),
        &cairo0_proof_path("fibonacci_2000.proof"),
        &cairo0_proof_path("fibonacci_2000_sec.proof"),
    );
    run_table_bench(
        &mut group,
        "fibonacci/5000",
        &cairo0_program_path("fibonacci_5000.json"),
        &cairo0_proof_path("fibonacci_5000.proof"),
        &cairo0_proof_path("fibonacci_5000_sec.proof"),
    );
    run_table_bench(
        &mut group,
        "fibonacci/20000",
        &cairo0_program_path("fibonacci_20000.json"),
        &cairo0_proof_path("fibonacci_20000.proof"),
        &cairo0_proof_path("fibonacci_20000_sec.proof"),
    );
}

fn run_table_bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    benchname: &str,
    program_path: &str,
    proof_path: &str,
    sec_proof_path: &str,
) {
    run_trace_bench(group, &format!("trace/{benchname}"), program_path);
    run_cairo_bench_and_measure_proof(
        group,
        &format!("prover/80_bits/{benchname}"),
        program_path,
        SecurityLevel::Conjecturable80Bits,
    );
    run_verifier_bench_with_security_level(
        group,
        &format!("verifier/80_bits/{benchname}"),
        proof_path,
        SecurityLevel::Conjecturable80Bits,
    );
    run_cairo_bench_and_measure_proof(
        group,
        &format!("prover/128_bits/{benchname}"),
        program_path,
        SecurityLevel::Conjecturable128Bits,
    );
    run_verifier_bench_with_security_level(
        group,
        &format!("verifier/128_bits/{benchname}"),
        sec_proof_path,
        SecurityLevel::Conjecturable128Bits,
    );
}

criterion_group!(benches, table_benches);
criterion_main!(benches);
