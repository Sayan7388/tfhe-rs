use crate::conformance::ParameterSetConformant;
use crate::core_crypto::prelude::*;
use crate::shortint::parameters::CiphertextConformanceParams;
use crate::shortint::{CarryModulus, MessageModulus};

use self::compressed_modulus_switched_glwe_ciphertext::CompressedModulusSwitchedGlweCiphertext;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct GlwePackedCiphertextList {
    pub compressed_modulus_switched_lwe_ciphertext:
        Vec<CompressedModulusSwitchedGlweCiphertext<u64>>,
    pub glwe_dimension: GlweDimension,
    pub polynomial_size: PolynomialSize,
    pub ciphertext_modulus: CiphertextModulus<u64>,
    pub message_modulus: MessageModulus,
    pub carry_modulus: CarryModulus,
    pub pbs_order: PBSOrder,
    pub lwe_per_glwe: usize,
    pub count: usize,
}

impl ParameterSetConformant for GlwePackedCiphertextList {
    type ParameterSet = CiphertextConformanceParams;

    fn is_conformant(&self, params: &CiphertextConformanceParams) -> bool {
        self.lwe_per_glwe <= self.polynomial_size.0
            && self.ciphertext_modulus == params.ct_params.ct_modulus
            && self.message_modulus == params.message_modulus
            && self.carry_modulus == params.carry_modulus
            && self.pbs_order == params.pbs_order
    }
}
