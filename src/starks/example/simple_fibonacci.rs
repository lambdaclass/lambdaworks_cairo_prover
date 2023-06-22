use lambdaworks_crypto::fiat_shamir::transcript::Transcript;
use lambdaworks_math::field::{
    element::FieldElement, fields::fft_friendly::stark_252_prime_field::Stark252PrimeField,
    traits::IsFFTField,
};

use crate::starks::{
    constraints::boundary::{BoundaryConstraint, BoundaryConstraints},
    context::AirContext,
    frame::Frame,
    trace::TraceTable,
    traits::AIR,
};

#[derive(Clone)]
pub struct FibonacciAIR {
    context: AirContext,
}

impl From<AirContext> for FibonacciAIR {
    fn from(context: AirContext) -> Self {
        Self { context }
    }
}

impl AIR for FibonacciAIR {
    type Field = Stark252PrimeField;
    type RAPChallenges = ();
    type PublicInput = ();

    fn composition_poly_degree_bound(&self) -> usize {
        self.context().trace_length
    }

    fn build_auxiliary_trace(
        &self,
        _main_trace: &TraceTable<Self::Field>,
        _rap_challenges: &Self::RAPChallenges,
        _public_input: &Self::PublicInput,
    ) -> TraceTable<Self::Field> {
        TraceTable::empty()
    }

    fn build_rap_challenges<T: Transcript>(&self, _transcript: &mut T) -> Self::RAPChallenges {}
    fn compute_transition(
        &self,
        frame: &Frame<Self::Field>,
        _rap_challenges: &Self::RAPChallenges,
    ) -> Vec<FieldElement<Self::Field>> {
        let first_row = frame.get_row(0);
        let second_row = frame.get_row(1);
        let third_row = frame.get_row(2);

        vec![third_row[0].clone() - second_row[0].clone() - first_row[0].clone()]
    }

    fn boundary_constraints(
        &self,
        _rap_challenges: &Self::RAPChallenges,
        _public_input: &Self::PublicInput,
    ) -> BoundaryConstraints<Self::Field> {
        let a0 = BoundaryConstraint::new_simple(0, FieldElement::<Self::Field>::one());
        let a1 = BoundaryConstraint::new_simple(1, FieldElement::<Self::Field>::one());

        BoundaryConstraints::from_constraints(vec![a0, a1])
    }

    fn number_auxiliary_rap_columns(&self) -> usize {
        0
    }

    fn context(&self) -> &AirContext {
        &self.context
    }
}

pub fn fibonacci_trace<F: IsFFTField>(
    initial_values: [FieldElement<F>; 2],
    trace_length: usize,
) -> TraceTable<F> {
    let mut ret: Vec<FieldElement<F>> = vec![];

    ret.push(initial_values[0].clone());
    ret.push(initial_values[1].clone());

    for i in 2..(trace_length) {
        ret.push(ret[i - 1].clone() + ret[i - 2].clone());
    }

    TraceTable::new_from_cols(&[ret])
}
