use crate::core_crypto::commons::generators::DeterministicSeeder;
use crate::core_crypto::prelude::ActivatedRandomGenerator;
use crate::integer::public_key::CompactPublicKey;
use crate::integer::CompressedCompactPublicKey;
use crate::shortint::glwe_compression::{
    CompressionParameters, GlweCompressionKey, GlweCompressionPrivateKeys, GlweDecompressionKey,
    SeededGlweCompressionKey, SeededGlweDecompressionKey,
};
use crate::shortint::{EncryptionKeyChoice, MessageModulus};
use crate::Error;
use concrete_csprng::seeders::Seed;
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_field_names)]
#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct IntegerConfig {
    pub(crate) block_parameters: crate::shortint::PBSParameters,
    pub(crate) wopbs_block_parameters: Option<crate::shortint::WopbsParameters>,
    pub(crate) compression_parameters: Option<CompressionParameters>,
}

impl IntegerConfig {
    pub(crate) fn new(
        block_parameters: crate::shortint::PBSParameters,
        wopbs_block_parameters: Option<crate::shortint::WopbsParameters>,
        compression_parameters: Option<CompressionParameters>,
    ) -> Self {
        Self {
            block_parameters,
            wopbs_block_parameters,
            compression_parameters,
        }
    }

    pub(in crate::high_level_api) fn default_big() -> Self {
        Self {
            block_parameters: crate::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS.into(),
            wopbs_block_parameters: None,
            compression_parameters: None,
        }
    }

    pub(in crate::high_level_api) fn default_small() -> Self {
        Self {
            block_parameters: crate::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_PBS_KS.into(),
            wopbs_block_parameters: None,
            compression_parameters: None,
        }
    }

    pub fn enable_wopbs(&mut self) {
        let wopbs_block_parameters = match self.block_parameters.encryption_key_choice() {
            EncryptionKeyChoice::Big => crate::shortint::parameters::parameters_wopbs_message_carry::WOPBS_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            EncryptionKeyChoice::Small=> panic!("WOPBS only support KS_PBS parameters")
        };

        self.wopbs_block_parameters = Some(wopbs_block_parameters);
    }

    pub fn enable_glwe_packing_compression(
        &mut self,
        compression_parameters: CompressionParameters,
    ) {
        self.compression_parameters = Some(compression_parameters);
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct IntegerClientKey {
    pub(crate) key: crate::integer::ClientKey,
    pub(crate) wopbs_block_parameters: Option<crate::shortint::WopbsParameters>,
    pub(crate) compression_key: Option<GlweCompressionPrivateKeys>,
}

impl IntegerClientKey {
    pub(crate) fn with_seed(config: IntegerConfig, seed: Seed) -> Self {
        assert!(
            (config.block_parameters.message_modulus().0) == 2 || config.block_parameters.message_modulus().0 == 4,
            "This API only supports parameters for which the MessageModulus is 2 or 4 (1 or 2 bits per block)",
        );
        let mut seeder = DeterministicSeeder::<ActivatedRandomGenerator>::new(seed);
        let cks = crate::shortint::engine::ShortintEngine::new_from_seeder(&mut seeder)
            .new_client_key(config.block_parameters.into());

        let compression_key = config
            .compression_parameters
            .map(|params| cks.new_compression_private_key(params));

        let key = crate::integer::ClientKey::from(cks);

        Self {
            key,
            wopbs_block_parameters: config.wopbs_block_parameters,
            compression_key,
        }
    }

    /// Deconstruct an [`IntegerClientKey`] into its constituents.
    pub fn into_raw_parts(
        self,
    ) -> (
        crate::integer::ClientKey,
        Option<crate::shortint::WopbsParameters>,
        Option<GlweCompressionPrivateKeys>,
    ) {
        let Self {
            key,
            wopbs_block_parameters,
            compression_key: compression,
        } = self;
        (key, wopbs_block_parameters, compression)
    }

    /// Construct a, [`IntegerClientKey`] from its constituents.
    ///
    /// # Panics
    ///
    /// Panics if the provided raw parts are not compatible with the provided parameters.
    pub fn from_raw_parts(
        key: crate::integer::ClientKey,
        wopbs_block_parameters: Option<crate::shortint::WopbsParameters>,
        compression_key: Option<GlweCompressionPrivateKeys>,
    ) -> Self {
        let shortint_cks: &crate::shortint::ClientKey = key.as_ref();
        if let Some(wop_params) = wopbs_block_parameters.as_ref() {
            assert_eq!(
                shortint_cks.parameters.message_modulus(),
                wop_params.message_modulus
            );
            assert_eq!(
                shortint_cks.parameters.carry_modulus(),
                wop_params.carry_modulus
            );
        }

        Self {
            key,
            wopbs_block_parameters,
            compression_key,
        }
    }

    pub(crate) fn block_parameters(&self) -> crate::shortint::parameters::PBSParameters {
        self.key.parameters()
    }
}

impl From<IntegerConfig> for IntegerClientKey {
    fn from(config: IntegerConfig) -> Self {
        assert!(
            (config.block_parameters.message_modulus().0) == 2 || config.block_parameters.message_modulus().0 == 4,
            "This API only supports parameters for which the MessageModulus is 2 or 4 (1 or 2 bits per block)",
        );

        let key = crate::integer::ClientKey::new(config.block_parameters);

        let compression_key = config
            .compression_parameters
            .map(|params| key.key.new_compression_private_key(params));

        Self {
            key,
            wopbs_block_parameters: config.wopbs_block_parameters,
            compression_key,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegerServerKey {
    pub(crate) key: crate::integer::ServerKey,
    pub(crate) wopbs_key: Option<crate::integer::wopbs::WopbsKey>,
    pub(crate) compression_key: Option<GlweCompressionKey>,
    pub(crate) decompression_key: Option<GlweDecompressionKey>,
}

impl IntegerServerKey {
    pub(in crate::high_level_api) fn new(client_key: &IntegerClientKey) -> Self {
        let cks = &client_key.key;

        let (compression_key, decompression_key) = client_key.compression_key.as_ref().map_or_else(
            || (None, None),
            |a| {
                let (compression_key, decompression_key) =
                    cks.key.new_compression_decompression_keys(a);
                (Some(compression_key), Some(decompression_key))
            },
        );

        let base_integer_key = crate::integer::ServerKey::new_radix_server_key(cks);
        let wopbs_key = client_key
            .wopbs_block_parameters
            .as_ref()
            .map(|wopbs_params| {
                crate::integer::wopbs::WopbsKey::new_wopbs_key(cks, &base_integer_key, wopbs_params)
            });

        Self {
            key: base_integer_key,
            wopbs_key,
            compression_key,
            decompression_key,
        }
    }

    pub(in crate::high_level_api) fn pbs_key(&self) -> &crate::integer::ServerKey {
        &self.key
    }

    pub(in crate::high_level_api) fn message_modulus(&self) -> MessageModulus {
        self.key.message_modulus()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegerCompressedServerKey {
    pub(crate) key: crate::integer::CompressedServerKey,
    pub(crate) compression_keys: Option<(SeededGlweCompressionKey, SeededGlweDecompressionKey)>,
}

impl IntegerCompressedServerKey {
    pub(in crate::high_level_api) fn new(client_key: &IntegerClientKey) -> Self {
        let integer_key = &client_key.key;
        assert!(
            client_key.wopbs_block_parameters.is_none(),
            "The configuration used to create the ClientKey \
                   had function evaluation on integers enabled.
                   This feature requires an additional key that is not
                   compressible. Thus, It is not possible
                   to create a CompressedServerKey.
                   "
        );
        let key = crate::integer::CompressedServerKey::new_radix_compressed_server_key(integer_key);

        let compression_keys = client_key.compression_key.as_ref().map_or_else(
            || None,
            |a| {
                Some(
                    client_key
                        .key
                        .key
                        .new_seeded_compression_decompression_keys(a),
                )
            },
        );

        Self {
            key,
            compression_keys,
        }
    }

    pub fn into_raw_parts(
        self,
    ) -> (
        crate::integer::CompressedServerKey,
        Option<(SeededGlweCompressionKey, SeededGlweDecompressionKey)>,
    ) {
        (self.key, self.compression_keys)
    }

    pub fn from_raw_parts(
        key: crate::integer::CompressedServerKey,
        compression_keys: Option<(SeededGlweCompressionKey, SeededGlweDecompressionKey)>,
    ) -> Self {
        Self {
            key,
            compression_keys,
        }
    }

    pub(in crate::high_level_api) fn decompress(&self) -> IntegerServerKey {
        let (compression_key, decompression_key) = self
            .compression_keys
            .as_ref()
            .map_or((None, None), |(comp, decomp)| {
                (Some(comp.decompress()), Some(decomp.decompress()))
            });

        IntegerServerKey {
            key: self.key.decompress(),
            wopbs_key: None,
            compression_key,
            decompression_key,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::high_level_api) struct IntegerCompactPublicKey {
    pub(in crate::high_level_api) key: CompactPublicKey,
}

impl IntegerCompactPublicKey {
    pub(in crate::high_level_api) fn new(client_key: &IntegerClientKey) -> Self {
        Self::try_new(client_key).expect("Incompatible parameters")
    }

    pub(in crate::high_level_api) fn try_new(client_key: &IntegerClientKey) -> Result<Self, Error> {
        let cks = &client_key.key;

        let key = CompactPublicKey::try_new(cks)?;

        Ok(Self { key })
    }

    pub fn into_raw_parts(self) -> CompactPublicKey {
        self.key
    }

    pub fn from_raw_parts(key: CompactPublicKey) -> Self {
        Self { key }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::high_level_api) struct IntegerCompressedCompactPublicKey {
    pub(in crate::high_level_api) key: CompressedCompactPublicKey,
}

impl IntegerCompressedCompactPublicKey {
    pub(in crate::high_level_api) fn new(client_key: &IntegerClientKey) -> Self {
        let cks = &client_key.key;

        let key = CompressedCompactPublicKey::new(cks);

        Self { key }
    }

    /// Deconstruct a [`IntegerCompressedCompactPublicKey`] into its constituents.
    pub fn into_raw_parts(self) -> CompressedCompactPublicKey {
        self.key
    }

    /// Construct a [`IntegerCompressedCompactPublicKey`] from its constituents.
    pub fn from_raw_parts(key: CompressedCompactPublicKey) -> Self {
        Self { key }
    }

    pub(in crate::high_level_api) fn decompress(&self) -> IntegerCompactPublicKey {
        IntegerCompactPublicKey {
            key: CompressedCompactPublicKey::decompress(&self.key),
        }
    }
}
