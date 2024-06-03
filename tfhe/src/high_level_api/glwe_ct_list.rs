use super::global_state::with_cpu_internal_keys;
use crate::core_crypto::commons::math::random::{Deserialize, Serialize};
use crate::high_level_api::integers::{FheIntId, FheUintId};
use crate::integer::ciphertext::{DataKind, Expandable, GlwePackable};
use crate::named::Named;
use crate::shortint::{Ciphertext, MessageModulus};
use crate::{FheBool, FheInt, FheUint};

impl<Id: FheUintId> GlwePackable for FheUint<Id> {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        message_modulus: MessageModulus,
        num_blocks: Option<usize>,
    ) -> DataKind {
        match self.ciphertext {
            super::integers::unsigned::inner::RadixCiphertext::Cpu(a) => {
                a.compact_into(messages, message_modulus, num_blocks)
            }
            #[cfg(feature = "gpu")]
            super::integers::unsigned::inner::RadixCiphertext::Cuda(_) => panic!(),
        }
    }
}

impl<Id: FheIntId> GlwePackable for FheInt<Id> {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        message_modulus: MessageModulus,
        num_blocks: Option<usize>,
    ) -> DataKind {
        match self.ciphertext {
            super::integers::signed::inner::RadixCiphertext::Cpu(a) => {
                a.compact_into(messages, message_modulus, num_blocks)
            }
            #[cfg(feature = "gpu")]
            super::integers::signed::inner::RadixCiphertext::Cuda(_) => panic!(),
        }
    }
}

impl GlwePackable for FheBool {
    fn compact_into(
        self,
        messages: &mut Vec<Ciphertext>,
        message_modulus: MessageModulus,
        num_blocks: Option<usize>,
    ) -> DataKind {
        match self.ciphertext {
            super::booleans::inner::InnerBoolean::Cpu(a) => {
                a.compact_into(messages, message_modulus, num_blocks)
            }
            #[cfg(feature = "gpu")]
            super::booleans::inner::InnerBoolean::Cuda(_) => panic!(),
        }
    }
}

pub struct GlwePackedCompressedCiphertextListBuilder {
    inner: crate::integer::ciphertext::GlwePackedCompressedCiphertextListBuilder,
}

impl GlwePackedCompressedCiphertextListBuilder {
    pub fn new(message_modulus: MessageModulus) -> Self {
        Self {
            inner: crate::integer::ciphertext::GlwePackedCompressedCiphertextListBuilder::new(
                message_modulus,
            ),
        }
    }

    pub fn push<T>(&mut self, value: T) -> &mut Self
    where
        T: GlwePackable,
    {
        self.inner.push(value);
        self
    }

    pub fn extend<T>(&mut self, values: impl Iterator<Item = T>) -> &mut Self
    where
        T: GlwePackable,
    {
        self.inner.extend(values);
        self
    }

    pub fn build(&self) -> GlwePackedCompressedCiphertextList {
        with_cpu_internal_keys(|key| {
            GlwePackedCompressedCiphertextList(
                self.inner.build(
                    key.compression_key
                        .as_ref()
                        .expect("Compression key not set in server key"),
                ),
            )
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GlwePackedCompressedCiphertextList(
    crate::integer::ciphertext::GlwePackedCompressedCiphertextList,
);

impl Named for GlwePackedCompressedCiphertextList {
    const NAME: &'static str = "high_level_api::CompactCiphertextList";
}

impl GlwePackedCompressedCiphertextList {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_kind_of(&self, index: usize) -> Option<crate::FheTypes> {
        Some(match self.0.get_kind_of(index)? {
            DataKind::Unsigned(n) => {
                let num_bits_per_block = self.0.packed_list.message_modulus.0.ilog2() as usize;
                let num_bits = n * num_bits_per_block;
                match num_bits {
                    2 => crate::FheTypes::Uint2,
                    4 => crate::FheTypes::Uint4,
                    6 => crate::FheTypes::Uint6,
                    8 => crate::FheTypes::Uint8,
                    10 => crate::FheTypes::Uint10,
                    12 => crate::FheTypes::Uint12,
                    14 => crate::FheTypes::Uint14,
                    16 => crate::FheTypes::Uint16,
                    32 => crate::FheTypes::Uint32,
                    64 => crate::FheTypes::Uint64,
                    128 => crate::FheTypes::Uint128,
                    160 => crate::FheTypes::Uint160,
                    256 => crate::FheTypes::Uint256,
                    _ => return None,
                }
            }
            DataKind::Signed(n) => {
                let num_bits_per_block = self.0.packed_list.message_modulus.0.ilog2() as usize;
                let num_bits = n * num_bits_per_block;
                match num_bits {
                    2 => crate::FheTypes::Int2,
                    4 => crate::FheTypes::Int4,
                    6 => crate::FheTypes::Int6,
                    8 => crate::FheTypes::Int8,
                    10 => crate::FheTypes::Int10,
                    12 => crate::FheTypes::Int12,
                    14 => crate::FheTypes::Int14,
                    16 => crate::FheTypes::Int16,
                    32 => crate::FheTypes::Int32,
                    64 => crate::FheTypes::Int64,
                    128 => crate::FheTypes::Int128,
                    160 => crate::FheTypes::Int160,
                    256 => crate::FheTypes::Int256,
                    _ => return None,
                }
            }
            DataKind::Boolean => crate::FheTypes::Bool,
        })
    }

    pub fn get<T>(&self, index: usize) -> Option<crate::Result<T>>
    where
        T: Expandable,
    {
        with_cpu_internal_keys(|key| {
            self.0.get(
                index,
                key.decompression_key
                    .as_ref()
                    .expect("Decompression key not set in server key"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::shortint::glwe_compression::COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS;
    use crate::{set_server_key, FheInt64, FheUint16, FheUint2, FheUint32};

    #[test]
    fn test_glwe_ct_list() {
        let config = crate::ConfigBuilder::default()
            .enable_glwe_packing_compression(COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS)
            .build();

        let ck = crate::ClientKey::generate(config);
        let sk = crate::ServerKey::new(&ck);

        set_server_key(sk);

        let ct1 = FheUint32::encrypt(17_u32, &ck);

        let ct2 = FheInt64::encrypt(-1i64, &ck);

        let ct3 = FheBool::encrypt(false, &ck);

        let ct4 = FheUint2::encrypt(3u8, &ck);

        let compressed_list = GlwePackedCompressedCiphertextListBuilder::new(MessageModulus(4))
            .push(ct1)
            .push(ct2)
            .push(ct3)
            .push(ct4)
            .build();

        let serialized = bincode::serialize(&compressed_list).unwrap();

        let compressed_list: GlwePackedCompressedCiphertextList =
            bincode::deserialize(&serialized).unwrap();

        {
            let a: FheUint32 = compressed_list.get(0).unwrap().unwrap();
            let b: FheInt64 = compressed_list.get(1).unwrap().unwrap();
            let c: FheBool = compressed_list.get(2).unwrap().unwrap();
            let d: FheUint2 = compressed_list.get(3).unwrap().unwrap();

            let a: u32 = a.decrypt(&ck);
            assert_eq!(a, 17);
            let b: i64 = b.decrypt(&ck);
            assert_eq!(b, -1);
            let c = c.decrypt(&ck);
            assert!(!c);
            let d: u8 = d.decrypt(&ck);
            assert_eq!(d, 3);

            assert!(compressed_list.get::<FheBool>(4).is_none());
        }

        {
            // Incorrect type
            assert!(compressed_list.get::<FheInt64>(0).unwrap().is_err());

            // Correct type but wrong number of bits
            assert!(compressed_list.get::<FheUint16>(0).unwrap().is_err());
        }
    }
}
