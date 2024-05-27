use super::CompressionParameters;
use crate::core_crypto::prelude::{
    allocate_and_generate_new_binary_glwe_secret_key, GlweSecretKeyOwned,
};
use crate::shortint::client_key::ClientKey;
use crate::shortint::engine::ShortintEngine;
use crate::shortint::{ClassicPBSParameters, EncryptionKeyChoice, PBSParameters};
use std::fmt::Debug;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GlweCompressionPrivateKeys {
    pub post_packing_ks_key: GlweSecretKeyOwned<u64>,
    pub params: CompressionParameters,
}

impl ClientKey {
    pub fn new_compression_private_key(
        &self,
        params: CompressionParameters,
    ) -> GlweCompressionPrivateKeys {
        let cks_params: ClassicPBSParameters = match self.parameters.pbs_parameters().unwrap() {
            PBSParameters::PBS(a) => a,
            PBSParameters::MultiBitPBS(_) => panic!("Unsupported"),
        };

        assert!(cks_params.encryption_key_choice == EncryptionKeyChoice::Big);

        let post_packing_ks_key = ShortintEngine::with_thread_local_mut(|engine| {
            allocate_and_generate_new_binary_glwe_secret_key(
                params.packing_ks_glwe_dimension,
                params.packing_ks_polynomial_size,
                &mut engine.secret_generator,
            )
        });

        GlweCompressionPrivateKeys {
            post_packing_ks_key,
            params,
        }
    }
}
