pub mod air;
pub mod cairo_run;
pub mod cairo_vm;
pub mod fri;
pub mod proof;
pub mod prover;
pub mod verifier;

use std::marker::PhantomData;

use air::traits::AIR;
use lambdaworks_crypto::{
    fiat_shamir::transcript::Transcript, merkle_tree::traits::IsMerkleTreeBackend,
};
use lambdaworks_fft::roots_of_unity::get_powers_of_primitive_root_coset;
use lambdaworks_math::{
    field::{
        element::FieldElement,
        fields::fft_friendly::stark_252_prime_field::Stark252PrimeField,
        traits::{IsFFTField, IsField},
    },
    traits::ByteConversion,
};
use sha3::{Digest, Sha3_256};

pub struct ProofConfig {
    pub count_queries: usize,
    pub blowup_factor: usize,
}

pub type PrimeField = Stark252PrimeField;
pub type FE = FieldElement<PrimeField>;

// TODO: change this to use more bits
pub fn transcript_to_field<F: IsField, T: Transcript>(transcript: &mut T) -> FieldElement<F> {
    let value: u64 = u64::from_be_bytes(transcript.challenge()[..8].try_into().unwrap());
    FieldElement::from(value)
}

pub fn transcript_to_usize<T: Transcript>(transcript: &mut T) -> usize {
    const CANT_BYTES_USIZE: usize = (usize::BITS / 8) as usize;
    let value = transcript.challenge()[..CANT_BYTES_USIZE]
        .try_into()
        .unwrap();
    usize::from_be_bytes(value)
}

pub fn sample_z_ood<F: IsField, T: Transcript>(
    lde_roots_of_unity_coset: &[FieldElement<F>],
    trace_roots_of_unity: &[FieldElement<F>],
    transcript: &mut T,
) -> FieldElement<F> {
    loop {
        let value: FieldElement<F> = transcript_to_field(transcript);
        if !lde_roots_of_unity_coset.iter().any(|x| x == &value)
            && !trace_roots_of_unity.iter().any(|x| x == &value)
        {
            return value;
        }
    }
}

pub fn batch_sample_challenges<F: IsFFTField, T: Transcript>(
    size: usize,
    transcript: &mut T,
) -> Vec<FieldElement<F>> {
    (0..size).map(|_| transcript_to_field(transcript)).collect()
}

pub struct Domain<F: IsFFTField> {
    root_order: u32,
    lde_roots_of_unity_coset: Vec<FieldElement<F>>,
    lde_root_order: u32,
    trace_primitive_root: FieldElement<F>,
    trace_roots_of_unity: Vec<FieldElement<F>>,
    coset_offset: FieldElement<F>,
    blowup_factor: usize,
    interpolation_domain_size: usize,
}

impl<F: IsFFTField> Domain<F> {
    fn new<A: AIR<Field = F>>(air: &A) -> Self {
        // Initial definitions
        let blowup_factor = air.options().blowup_factor as usize;
        let coset_offset = FieldElement::<F>::from(air.options().coset_offset);
        let interpolation_domain_size = air.context().trace_length;
        let root_order = air.context().trace_length.trailing_zeros();
        // * Generate Coset
        let trace_primitive_root = F::get_primitive_root_of_unity(root_order as u64).unwrap();
        let trace_roots_of_unity = get_powers_of_primitive_root_coset(
            root_order as u64,
            interpolation_domain_size,
            &FieldElement::<F>::one(),
        )
        .unwrap();

        let lde_root_order = (air.context().trace_length * blowup_factor).trailing_zeros();
        let lde_roots_of_unity_coset = get_powers_of_primitive_root_coset(
            lde_root_order as u64,
            air.context().trace_length * blowup_factor,
            &coset_offset,
        )
        .unwrap();

        Self {
            root_order,
            lde_roots_of_unity_coset,
            lde_root_order,
            trace_primitive_root,
            trace_roots_of_unity,
            blowup_factor,
            coset_offset,
            interpolation_domain_size,
        }
    }
}

/// A Merkle tree backend for vectors of field elements.
/// This is used by the Stark prover to commit to
/// multiple trace columns using a single Merkle tree.
#[derive(Clone)]
pub struct BatchStarkProverBackend<F> {
    phantom: PhantomData<F>,
}

impl<F> Default for BatchStarkProverBackend<F> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<F> IsMerkleTreeBackend for BatchStarkProverBackend<F>
where
    F: IsField,
    FieldElement<F>: ByteConversion,
{
    type Node = [u8; 32];
    type Data = Vec<FieldElement<F>>;

    fn hash_data(&self, input: &Vec<FieldElement<F>>) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        for element in input.iter() {
            hasher.update(element.to_bytes_be());
        }
        let mut result_hash = [0_u8; 32];
        result_hash.copy_from_slice(&hasher.finalize());
        result_hash
    }

    fn hash_new_parent(&self, left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(left);
        hasher.update(right);
        let mut result_hash = [0_u8; 32];
        result_hash.copy_from_slice(&hasher.finalize());
        result_hash
    }
}
