use super::{DataKind, Expandable, RadixCiphertext, SignedRadixCiphertext};
use crate::integer::BooleanBlock;
use crate::shortint::ciphertext::GlwePackedCiphertextList;
use crate::shortint::glwe_compression::{GlweCompressionKey, GlweDecompressionKey};
use crate::shortint::{Ciphertext, MessageModulus};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub trait GlwePackable {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        message_modulus: MessageModulus,
        num_blocks: Option<usize>,
    ) -> DataKind;
}

impl GlwePackable for BooleanBlock {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        _message_modulus: MessageModulus,
        _num_blocks: Option<usize>,
    ) -> DataKind {
        messages.push(self.0);
        DataKind::Boolean
    }
}

impl GlwePackable for RadixCiphertext {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        _message_modulus: MessageModulus,
        _num_blocks: Option<usize>,
    ) -> DataKind {
        let num_blocks = self.blocks.len();

        for block in self.blocks {
            messages.push(block);
        }

        DataKind::Unsigned(num_blocks)
    }
}

impl GlwePackable for SignedRadixCiphertext {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        _message_modulus: MessageModulus,
        _num_blocks: Option<usize>,
    ) -> DataKind {
        let num_blocks = self.blocks.len();

        for block in self.blocks {
            messages.push(block);
        }

        DataKind::Signed(num_blocks)
    }
}

pub struct GlwePackedCompressedCiphertextListBuilder {
    pub(crate) ciphertexts: Vec<Ciphertext>,
    pub(crate) info: Vec<DataKind>,
    pub(crate) message_modulus: MessageModulus,
}

impl GlwePackedCompressedCiphertextListBuilder {
    pub fn new(message_modulus: MessageModulus) -> Self {
        Self {
            ciphertexts: vec![],
            info: vec![],
            message_modulus,
        }
    }

    pub fn push<T>(&mut self, data: T) -> &mut Self
    where
        T: GlwePackable,
    {
        let n = self.ciphertexts.len();
        let kind = data.compact_into(&mut self.ciphertexts, self.message_modulus, None);
        assert_eq!(n + kind.num_blocks(), self.ciphertexts.len());

        if kind.num_blocks() != 0 {
            self.info.push(kind);
        }

        self
    }

    pub fn extend<T>(&mut self, values: impl Iterator<Item = T>) -> &mut Self
    where
        T: GlwePackable,
    {
        for value in values {
            self.push(value);
        }
        self
    }

    pub fn build(&self, comp_key: &GlweCompressionKey) -> GlwePackedCompressedCiphertextList {
        let packed_list = comp_key.pack_lwe_ciphertexts_into_glwes(&self.ciphertexts);

        GlwePackedCompressedCiphertextList {
            packed_list,
            info: self.info.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GlwePackedCompressedCiphertextList {
    pub(crate) packed_list: GlwePackedCiphertextList,
    info: Vec<DataKind>,
}

impl GlwePackedCompressedCiphertextList {
    pub fn len(&self) -> usize {
        self.info.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn blocks_of(
        &self,
        index: usize,
        decomp_key: &GlweDecompressionKey,
    ) -> Option<(Vec<Ciphertext>, DataKind)> {
        let preceding_infos = self.info.get(..index)?;
        let current_info = self.info.get(index).copied()?;

        let start_block_index: usize = preceding_infos
            .iter()
            .copied()
            .map(DataKind::num_blocks)
            .sum();

        let end_block_index = start_block_index + current_info.num_blocks();

        Some((
            (start_block_index..end_block_index)
                .into_par_iter()
                .map(|i| decomp_key.unpack(&self.packed_list, i))
                .collect(),
            current_info,
        ))
    }

    pub fn get_kind_of(&self, index: usize) -> Option<DataKind> {
        self.info.get(index).copied()
    }

    pub fn get<T>(
        &self,
        index: usize,
        decomp_key: &GlweDecompressionKey,
    ) -> Option<crate::Result<T>>
    where
        T: Expandable,
    {
        self.blocks_of(index, decomp_key)
            .map(|(blocks, kind)| T::from_expanded_blocks(blocks, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integer::{gen_keys_radix, RadixCiphertext};
    use crate::shortint::glwe_compression::COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS;
    use crate::shortint::parameters::PARAM_MESSAGE_2_CARRY_2_KS_PBS;

    #[test]
    fn test_heterogeneous_glwe_compression_ci_run_filter() {
        // Generate the client key and the server key:

        let num_blocks = 2;

        let (cks, sks) = gen_keys_radix(PARAM_MESSAGE_2_CARRY_2_KS_PBS, num_blocks);

        let private_compression_key =
            cks.new_compression_private_key(COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS);

        let (compression_key, decompression_key) =
            cks.new_compression_decompression_keys(&private_compression_key);

        let ct1 = cks.encrypt(3_u32);

        let ct2 = cks.encrypt_signed(-2);

        let ct3 = cks.encrypt_bool(true);

        let compressed = GlwePackedCompressedCiphertextListBuilder::new(sks.message_modulus())
            .push(ct1)
            .push(ct2)
            .push(ct3)
            .build(&compression_key);

        let a = compressed.blocks_of(0, &decompression_key).unwrap();

        let decrypted: u32 = cks.decrypt(&RadixCiphertext::from_expanded_blocks(a.0, a.1).unwrap());
        assert_eq!(decrypted, 3_u32);

        let b = compressed.blocks_of(1, &decompression_key).unwrap();

        let decrypted: i32 =
            cks.decrypt_signed(&SignedRadixCiphertext::from_expanded_blocks(b.0, b.1).unwrap());

        assert_eq!(decrypted, -2);

        let c = compressed.blocks_of(2, &decompression_key).unwrap();

        assert!(cks.decrypt_bool(&BooleanBlock::from_expanded_blocks(c.0, c.1).unwrap()));
    }
}
