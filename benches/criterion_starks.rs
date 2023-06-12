use std::time::Duration;

use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use lambdaworks_stark::{
    air::{
        cairo_air::air::{CairoAIR, PublicInputs},
        context::ProofOptions,
    },
    cairo_run::{cairo_layout::CairoLayout, run::run_program},
    prover::prove,
};

pub mod functions;
pub mod util;

fn cairo_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("CAIRO");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    run_cairo_bench(
        &mut group,
        "fibonacci/10",
        &program_path("fibonacci_10.json"),
    );
    run_cairo_bench(
        &mut group,
        "fibonacci/30",
        &program_path("fibonacci_30.json"),
    );
}

fn program_path(program_name: &str) -> String {
    const CARGO_DIR: &str = env!("CARGO_MANIFEST_DIR");
    const PROGRAM_BASE_REL_PATH: &str = "/src/cairo_vm/test_data/";
    let program_base_path = CARGO_DIR.to_string() + PROGRAM_BASE_REL_PATH;
    program_base_path + program_name
}

fn run_cairo_bench(group: &mut BenchmarkGroup<'_, WallTime>, benchname: &str, program_path: &str) {
    let (register_states, memory, program_size) =
        run_program(None, CairoLayout::Plain, program_path).unwrap();

    let proof_options = ProofOptions {
        blowup_factor: 4,
        fri_number_of_queries: 5,
        coset_offset: 3,
    };

    let mut pub_inputs = PublicInputs::from_regs_and_mem(&register_states, &memory, program_size);

    // This should be auto calculated
    // First pad is needed by the prover to validate the program bytecode
    let first_pad = (pub_inputs.program.len() >> 2) + 1;
    // Then we need to pad it to next power of two
    let padded_trace_length = (register_states.steps() + first_pad).next_power_of_two();

    let cairo_air = CairoAIR::new(proof_options, padded_trace_length, register_states.steps());

    group.bench_function(benchname, |bench| {
        bench.iter(|| {
            black_box(
                // TO DO: We should change the api to avoid consuming the states and the memory, so we don't have to clone
                prove(
                    &(register_states.clone(), memory.clone()),
                    &cairo_air,
                    &mut pub_inputs,
                )
                .unwrap(),
            )
        });
    });
}

criterion_group!(benches, cairo_benches);
criterion_main!(benches);
