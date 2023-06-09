use lambdaworks_crypto::fiat_shamir::transcript::Transcript;
use lambdaworks_math::{
    field::{element::FieldElement, traits::IsFFTField},
    polynomial::Polynomial,
};

use crate::prover::ProvingError;

use super::{
    constraints::boundary::BoundaryConstraints,
    context::{AirContext, ProofOptions},
    frame::Frame,
    trace::TraceTable,
};
use crate::get_powers_of_primitive_root_coset;
/// AIR is a representation of the Constraints
pub trait AIR: Clone {
    type Field: IsFFTField;
    type RawTrace;
    type RAPChallenges;
    type PublicInput;

    fn build_main_trace(
        &self,
        raw_trace: &Self::RawTrace,
        public_input: &mut Self::PublicInput,
    ) -> Result<TraceTable<Self::Field>, ProvingError>;

    fn build_auxiliary_trace(
        &self,
        main_trace: &TraceTable<Self::Field>,
        rap_challenges: &Self::RAPChallenges,
        public_input: &Self::PublicInput,
    ) -> TraceTable<Self::Field>;

    fn build_rap_challenges<T: Transcript>(&self, transcript: &mut T) -> Self::RAPChallenges;

    fn number_auxiliary_rap_columns(&self) -> usize;

    fn composition_poly_degree_bound(&self) -> usize;

    fn compute_transition(
        &self,
        frame: &Frame<Self::Field>,
        rap_challenges: &Self::RAPChallenges,
    ) -> Vec<FieldElement<Self::Field>>;

    fn boundary_constraints(
        &self,
        rap_challenges: &Self::RAPChallenges,
        public_input: &Self::PublicInput,
    ) -> BoundaryConstraints<Self::Field>;

    fn transition_exemptions(&self) -> Vec<Polynomial<FieldElement<Self::Field>>> {
        let trace_length = self.context().trace_length;
        let roots_of_unity_order = trace_length.trailing_zeros();
        let roots_of_unity = get_powers_of_primitive_root_coset(
            roots_of_unity_order as u64,
            self.context().trace_length,
            &FieldElement::<Self::Field>::one(),
        )
        .unwrap();
        let root_of_unity_len = roots_of_unity.len();

        let x = Polynomial::new_monomial(FieldElement::one(), 1);

        self.context()
            .transition_exemptions
            .iter()
            .take(self.context().num_transition_constraints)
            .map(|cant_take| {
                roots_of_unity
                    .iter()
                    .take(root_of_unity_len)
                    .rev()
                    .take(*cant_take)
                    .fold(
                        Polynomial::new_monomial(FieldElement::one(), 0),
                        |acc, root| acc * (&x - root),
                    )
            })
            .collect()
    }
    fn context(&self) -> &AirContext;

    fn options(&self) -> &ProofOptions {
        &self.context().options
    }

    fn blowup_factor(&self) -> u8 {
        self.options().blowup_factor
    }

    fn num_transition_constraints(&self) -> usize {
        self.context().num_transition_constraints
    }
}
