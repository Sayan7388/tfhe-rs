use super::{GlweCompressionKey, GlweCompressionPrivateKeys, GlweDecompressionKey};
use crate::core_crypto::prelude::{
    allocate_and_generate_new_seeded_lwe_packing_keyswitch_key,
    par_allocate_and_generate_new_seeded_lwe_bootstrap_key,
    par_convert_standard_lwe_bootstrap_key_to_fourier, FourierLweBootstrapKey,
    SeededLweBootstrapKeyOwned, SeededLwePackingKeyswitchKey,
};
use crate::shortint::client_key::ClientKey;
use crate::shortint::engine::ShortintEngine;
use crate::shortint::parameters::PolynomialSize;
use crate::shortint::server_key::ShortintBootstrappingKey;
use crate::shortint::{ClassicPBSParameters, EncryptionKeyChoice, PBSParameters};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeededGlweCompressionKey {
    pub packing_key_switching_key: SeededLwePackingKeyswitchKey<Vec<u64>>,
    pub lwe_per_glwe: usize,
    pub polynomial_size: PolynomialSize,
}

impl SeededGlweCompressionKey {
    pub fn decompress(&self) -> GlweCompressionKey {
        let packing_key_switching_key = self
            .packing_key_switching_key
            .as_view()
            .decompress_into_lwe_packing_keyswitch_key();

        GlweCompressionKey {
            packing_key_switching_key,
            lwe_per_glwe: self.lwe_per_glwe,
            polynomial_size: self.polynomial_size,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeededGlweDecompressionKey {
    pub blind_rotate_key: SeededLweBootstrapKeyOwned<u64>,
    pub lwe_per_glwe: usize,
}

impl SeededGlweDecompressionKey {
    pub fn decompress(&self) -> GlweDecompressionKey {
        let blind_rotate_key = self
            .blind_rotate_key
            .as_view()
            .par_decompress_into_lwe_bootstrap_key();

        let mut fourier_bsk = FourierLweBootstrapKey::new(
            blind_rotate_key.input_lwe_dimension(),
            blind_rotate_key.glwe_size(),
            blind_rotate_key.polynomial_size(),
            blind_rotate_key.decomposition_base_log(),
            blind_rotate_key.decomposition_level_count(),
        );

        // Conversion to fourier domain
        par_convert_standard_lwe_bootstrap_key_to_fourier(&blind_rotate_key, &mut fourier_bsk);

        GlweDecompressionKey {
            blind_rotate_key: ShortintBootstrappingKey::Classic(fourier_bsk),
            lwe_per_glwe: self.lwe_per_glwe,
        }
    }
}

impl ClientKey {
    pub fn new_seeded_compression_decompression_keys(
        &self,
        private_compression_key: &GlweCompressionPrivateKeys,
    ) -> (SeededGlweCompressionKey, SeededGlweDecompressionKey) {
        let cks_params: ClassicPBSParameters = match self.parameters.pbs_parameters().unwrap() {
            PBSParameters::PBS(a) => a,
            PBSParameters::MultiBitPBS(_) => panic!("Unsupported"),
        };

        let params = &private_compression_key.params;

        assert!(cks_params.encryption_key_choice == EncryptionKeyChoice::Big);

        let packing_key_switching_key = ShortintEngine::with_thread_local_mut(|engine| {
            allocate_and_generate_new_seeded_lwe_packing_keyswitch_key(
                &self.large_lwe_secret_key(),
                &private_compression_key.post_packing_ks_key,
                params.packing_ks_base_log,
                params.packing_ks_level,
                params.packing_ks_key_noise_distribution,
                self.parameters.ciphertext_modulus(),
                &mut engine.seeder,
            )
        });

        let glwe_compression_key = SeededGlweCompressionKey {
            packing_key_switching_key,
            lwe_per_glwe: params.lwe_per_glwe,
            polynomial_size: cks_params.polynomial_size,
        };

        let blind_rotate_key = ShortintEngine::with_thread_local_mut(|engine| {
            par_allocate_and_generate_new_seeded_lwe_bootstrap_key(
                &private_compression_key
                    .post_packing_ks_key
                    .as_lwe_secret_key(),
                &self.glwe_secret_key,
                private_compression_key.params.br_base_log,
                private_compression_key.params.br_level,
                self.parameters.glwe_noise_distribution(),
                self.parameters.ciphertext_modulus(),
                &mut engine.seeder,
            )
        });

        let glwe_decompression_key = SeededGlweDecompressionKey {
            blind_rotate_key,
            lwe_per_glwe: params.lwe_per_glwe,
        };

        (glwe_compression_key, glwe_decompression_key)
    }
}
