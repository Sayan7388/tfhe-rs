use super::GlweCompressionPrivateKeys;
use crate::core_crypto::prelude::{
    allocate_and_generate_new_lwe_packing_keyswitch_key, GlweSize, LwePackingKeyswitchKey,
};
use crate::shortint::client_key::ClientKey;
use crate::shortint::engine::ShortintEngine;
use crate::shortint::parameters::PolynomialSize;
use crate::shortint::server_key::ShortintBootstrappingKey;
use crate::shortint::{ClassicPBSParameters, EncryptionKeyChoice, PBSParameters};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlweCompressionKey {
    pub packing_key_switching_key: LwePackingKeyswitchKey<Vec<u64>>,
    pub lwe_per_glwe: usize,
    pub polynomial_size: PolynomialSize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlweDecompressionKey {
    pub blind_rotate_key: ShortintBootstrappingKey,
    pub lwe_per_glwe: usize,
}

impl GlweDecompressionKey {
    pub fn out_glwe_size(&self) -> GlweSize {
        self.blind_rotate_key.glwe_size()
    }

    pub fn out_polynomial_size(&self) -> PolynomialSize {
        self.blind_rotate_key.polynomial_size()
    }
}

impl ClientKey {
    pub fn new_compression_decompression_keys(
        &self,
        private_compression_key: &GlweCompressionPrivateKeys,
    ) -> (GlweCompressionKey, GlweDecompressionKey) {
        let cks_params: ClassicPBSParameters = match self.parameters.pbs_parameters().unwrap() {
            PBSParameters::PBS(a) => a,
            PBSParameters::MultiBitPBS(_) => panic!("Unsupported"),
        };

        let params = &private_compression_key.params;

        assert!(cks_params.encryption_key_choice == EncryptionKeyChoice::Big);

        let packing_key_switching_key = ShortintEngine::with_thread_local_mut(|engine| {
            allocate_and_generate_new_lwe_packing_keyswitch_key(
                &self.large_lwe_secret_key(),
                &private_compression_key.post_packing_ks_key,
                params.packing_ks_base_log,
                params.packing_ks_level,
                params.packing_ks_key_noise_distribution,
                self.parameters.ciphertext_modulus(),
                &mut engine.encryption_generator,
            )
        });

        let glwe_compression_key = GlweCompressionKey {
            packing_key_switching_key,
            lwe_per_glwe: params.lwe_per_glwe,
            polynomial_size: cks_params.polynomial_size,
        };

        let blind_rotate_key = ShortintEngine::with_thread_local_mut(|engine| {
            ShortintBootstrappingKey::Classic(
                engine.new_classic_bootstrapping_key(
                    &private_compression_key
                        .post_packing_ks_key
                        .as_lwe_secret_key(),
                    &self.glwe_secret_key,
                    self.parameters.glwe_noise_distribution(),
                    private_compression_key.params.br_base_log,
                    private_compression_key.params.br_level,
                    self.parameters.ciphertext_modulus(),
                ),
            )
        });

        let glwe_decompression_key = GlweDecompressionKey {
            blind_rotate_key,
            lwe_per_glwe: params.lwe_per_glwe,
        };

        (glwe_compression_key, glwe_decompression_key)
    }
}
