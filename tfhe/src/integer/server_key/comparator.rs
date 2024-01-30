use rayon::prelude::*;

use super::ServerKey;
use crate::core_crypto::prelude::Plaintext;
use crate::integer::block_decomposition::{BlockDecomposer, DecomposableInto};
use crate::integer::ciphertext::boolean_value::BooleanBlock;
use crate::integer::ciphertext::IntegerRadixCiphertext;
use crate::shortint::server_key::LookupTableOwned;
use crate::shortint::Ciphertext;

/// Used for compare_blocks_with_zero
#[derive(Clone, Copy)]
pub(crate) enum ZeroComparisonType {
    // We want to compare with zeros for equality (==)
    Equality,
    // We want to compare with zeros for difference (!=)
    Difference,
}

/// Simple enum to select whether we are looking for the min or the max
#[derive(Clone, Copy)]
enum MinMaxSelector {
    Max,
    Min,
}

/// This function encodes how to reduce the ordering of two blocks.
///
/// It is used to generate the necessary lookup table.
///
/// `x` is actually two ordering values packed
/// where in the first 2 lsb bits there is the ordering value of the less significant block,
/// and the 2 msb bits of x contains the ordering of the more significant block.
fn reduce_two_orderings_function(x: u64) -> u64 {
    let msb = (x >> 2) & 3;
    let lsb = x & 3;

    if msb == Comparator::IS_EQUAL {
        lsb
    } else {
        msb
    }
}

/// struct to compare integers
///
/// This struct keeps in memory the LUTs that are used
/// during the comparisons and min/max algorithms
pub struct Comparator<'a> {
    pub(crate) server_key: &'a ServerKey,
    // lut to get the sign of (a - b), used as the backbone of comparisons
    sign_lut: LookupTableOwned,
    // lut used to reduce 2 comparison blocks into 1
    comparison_reduction_lut: LookupTableOwned,
    comparison_result_to_offset_lut: LookupTableOwned,
    lhs_lut: LookupTableOwned,
    rhs_lut: LookupTableOwned,
}

impl<'a> Comparator<'a> {
    pub(crate) const IS_INFERIOR: u64 = 0;
    const IS_EQUAL: u64 = 1;
    pub(crate) const IS_SUPERIOR: u64 = 2;

    /// Creates a new Comparator for the given ServerKey
    ///
    /// # Panics
    ///
    /// panics if the message space + carry space is inferior to 4 bits
    pub fn new(server_key: &'a ServerKey) -> Self {
        assert!(
            server_key.key.message_modulus.0 * server_key.key.carry_modulus.0 >= 16,
            "At least 4 bits of space (message + carry) are required to be able to do comparisons"
        );

        let message_modulus = server_key.key.message_modulus.0 as u64;
        let sign_lut = server_key.key.generate_lookup_table(|x| u64::from(x != 0));

        let comparison_reduction_lut = server_key
            .key
            .generate_lookup_table(reduce_two_orderings_function);

        let comparison_result_to_offset_lut = server_key.key.generate_lookup_table(|sign| {
            if sign == Self::IS_INFERIOR {
                message_modulus
            } else {
                0
            }
        });

        let lhs_lut = server_key
            .key
            .generate_lookup_table(|x| if x < message_modulus { x } else { 0 });

        let rhs_lut = server_key.key.generate_lookup_table(|x| {
            if x >= message_modulus {
                x - message_modulus
            } else {
                0
            }
        });

        Self {
            server_key,
            sign_lut,
            comparison_reduction_lut,
            comparison_result_to_offset_lut,
            lhs_lut,
            rhs_lut,
        }
    }

    /// This function is used to compare two blocks
    /// of two signed radix ciphertext which hold the 'sign' bit
    /// (so the two most significant blocks).
    ///
    /// As for the blocks which holds the sign bit, the comparison
    /// is different than for regular blocks.
    fn compare_blocks_with_sign_bit(
        &self,
        lhs_block: &crate::shortint::Ciphertext,
        rhs_block: &crate::shortint::Ciphertext,
    ) -> crate::shortint::Ciphertext {
        let sign_bit_pos = self.server_key.key.message_modulus.0.ilog2() - 1;
        let lut = self.server_key.key.generate_lookup_table_bivariate(|x, y| {
            let x_sign_bit = x >> sign_bit_pos;
            let y_sign_bit = y >> sign_bit_pos;
            // The block that has its sign bit set is going
            // to be ordered as 'greater' by the cmp fn.
            // However, we are dealing with signed number,
            // so in reality, it is the smaller of the two.
            // i.e the cmp result is inversed
            if x_sign_bit == y_sign_bit {
                // Both have either sign bit set or unset,
                // cmp will give correct result
                match x.cmp(&y) {
                    std::cmp::Ordering::Less => Self::IS_INFERIOR,
                    std::cmp::Ordering::Equal => Self::IS_EQUAL,
                    std::cmp::Ordering::Greater => Self::IS_SUPERIOR,
                }
            } else {
                match x.cmp(&y) {
                    std::cmp::Ordering::Less => Self::IS_SUPERIOR,
                    std::cmp::Ordering::Equal => Self::IS_EQUAL,
                    std::cmp::Ordering::Greater => Self::IS_INFERIOR,
                }
            }
        });

        self.server_key
            .key
            .unchecked_apply_lookup_table_bivariate(lhs_block, rhs_block, &lut)
    }

    /// Takes a chunk of 2 ciphertexts and packs them together in a new ciphertext
    ///
    /// The first element of the chunk are the low bits, the second are the high bits
    ///
    /// This requires the block parameters to have enough room for two ciphertexts,
    /// so at least as many carry modulus as the message modulus
    ///
    /// Expects the carry buffer to be empty
    fn pack_block_chunk(
        &self,
        chunk: &[crate::shortint::Ciphertext],
    ) -> crate::shortint::Ciphertext {
        self.server_key.pack_block_chunk(chunk)
    }

    // lhs will be assigned
    // - 0 if lhs < rhs
    // - 1 if lhs == rhs
    // - 2 if lhs > rhs
    fn compare_block_assign(
        &self,
        lhs: &mut crate::shortint::Ciphertext,
        rhs: &crate::shortint::Ciphertext,
    ) {
        // When rhs > lhs, the subtraction will overflow, and the bit of padding will be set to 1
        // meaning that the output of the pbs will be the negative (modulo message space)
        //
        // Example:
        // lhs: 1, rhs: 3, message modulus: 4, carry modulus 4
        // lhs - rhs = -2 % (4 * 4) = 14 = 1|1110 (padding_bit|b4b3b2b1)
        // Since there was an overflow the bit of padding is 1 and not 0.
        // When applying the LUT for an input value of 14 we would expect 1,
        // but since the bit of padding is 1, we will get -1 modulus our message space,
        // so (-1) % (4 * 4) = 15 = 1|1111
        // We then add one and get 0 = 0|0000

        // Here we need the true lwe sub, not the one that comes from shortint.
        crate::core_crypto::algorithms::lwe_ciphertext_sub_assign(&mut lhs.ct, &rhs.ct);
        lhs.set_noise_level(lhs.noise_level() + rhs.noise_level());
        self.server_key
            .key
            .apply_lookup_table_assign(lhs, &self.sign_lut);

        // Here Lhs can have the following values: (-1) % (message modulus * carry modulus), 0, 1
        // So the output values after the addition will be: 0, 1, 2
        self.server_key.key.unchecked_scalar_add_assign(lhs, 1);
    }

    // lhs will be assigned
    // - 0 if lhs < rhs
    // - 1 if lhs == rhs
    // - 2 if lhs > rhs
    fn scalar_compare_block_assign(&self, lhs: &mut crate::shortint::Ciphertext, scalar: u8) {
        // Same logic as compare_block_assign
        // but rhs is a scalar
        let delta =
            (1u64 << (u64::BITS as u64 - 1)) / (lhs.carry_modulus.0 * lhs.message_modulus.0) as u64;
        let plaintext = Plaintext((scalar as u64) * delta);
        crate::core_crypto::algorithms::lwe_ciphertext_plaintext_sub_assign(&mut lhs.ct, plaintext);
        self.server_key
            .key
            .apply_lookup_table_assign(lhs, &self.sign_lut);

        self.server_key.key.unchecked_scalar_add_assign(lhs, 1);
    }

    fn reduce_two_sign_blocks_assign(
        &self,
        msb_sign: &mut crate::shortint::Ciphertext,
        lsb_sign: &crate::shortint::Ciphertext,
    ) {
        // We don't use pack_block_assign as the offset '4' does not depend on params
        self.server_key.key.unchecked_scalar_mul_assign(msb_sign, 4);
        self.server_key.key.unchecked_add_assign(msb_sign, lsb_sign);

        self.server_key
            .key
            .apply_lookup_table_assign(msb_sign, &self.comparison_reduction_lut);
    }

    /// Reduces a vec containing shortint blocks that encrypts a sign
    /// (inferior, equal, superior) to one single shortint block containing the
    /// final sign
    fn reduce_signs<F>(
        &self,
        mut sign_blocks: Vec<Ciphertext>,
        sign_result_handler_fn: F,
    ) -> Ciphertext
    where
        F: Fn(u64) -> u64,
    {
        while sign_blocks.len() > 2 {
            let mut sign_blocks_2: Vec<_> = sign_blocks
                .chunks_exact(2)
                .map(|chunk| {
                    let (low, high) = (&chunk[0], &chunk[1]);
                    let mut high = high.clone();
                    self.reduce_two_sign_blocks_assign(&mut high, low);
                    high
                })
                .collect();

            if (sign_blocks.len() % 2) == 1 {
                sign_blocks_2.push(sign_blocks[sign_blocks.len() - 1].clone());
            }

            std::mem::swap(&mut sign_blocks_2, &mut sign_blocks);
        }

        let final_lut = self.server_key.key.generate_lookup_table(|x| {
            let final_sign = reduce_two_orderings_function(x);
            sign_result_handler_fn(final_sign)
        });

        // We don't use pack_block_assign as the offset '4' does not depend on params
        let mut result = self.server_key.key.unchecked_scalar_mul(&sign_blocks[1], 4);
        self.server_key
            .key
            .unchecked_add_assign(&mut result, &sign_blocks[0]);
        self.server_key
            .key
            .apply_lookup_table_assign(&mut result, &final_lut);
        result
    }

    /// Reduces a vec containing shortint blocks that encrypts a sign
    /// (inferior, equal, superior) to one single shortint block containing the
    /// final sign
    fn reduce_signs_parallelized<F>(
        &self,
        mut sign_blocks: Vec<crate::shortint::Ciphertext>,
        sign_result_handler_fn: F,
    ) -> crate::shortint::Ciphertext
    where
        F: Fn(u64) -> u64,
    {
        let mut sign_blocks_2 = Vec::with_capacity(sign_blocks.len() / 2);
        while sign_blocks.len() > 2 {
            sign_blocks
                .par_chunks_exact(2)
                .map(|chunk| {
                    let (low, high) = (&chunk[0], &chunk[1]);
                    let mut high = high.clone();
                    self.reduce_two_sign_blocks_assign(&mut high, low);
                    high
                })
                .collect_into_vec(&mut sign_blocks_2);

            if (sign_blocks.len() % 2) == 1 {
                sign_blocks_2.push(sign_blocks[sign_blocks.len() - 1].clone());
            }

            std::mem::swap(&mut sign_blocks_2, &mut sign_blocks);
        }

        if sign_blocks.len() == 2 {
            let final_lut = self.server_key.key.generate_lookup_table(|x| {
                let final_sign = reduce_two_orderings_function(x);
                sign_result_handler_fn(final_sign)
            });
            // We don't use pack_block_assign as the offset '4' does not depend on params
            let mut result = self.server_key.key.unchecked_scalar_mul(&sign_blocks[1], 4);
            self.server_key
                .key
                .unchecked_add_assign(&mut result, &sign_blocks[0]);
            self.server_key
                .key
                .apply_lookup_table_assign(&mut result, &final_lut);
            result
        } else {
            let final_lut = self
                .server_key
                .key
                .generate_lookup_table(sign_result_handler_fn);
            self.server_key
                .key
                .apply_lookup_table(&sign_blocks[0], &final_lut)
        }
    }

    /// returns:
    ///
    /// - 0 if lhs < rhs
    /// - 1 if lhs == rhs
    /// - 2 if lhs > rhs
    ///
    /// Expects the carry buffers to be empty
    ///
    /// Requires that the RadixCiphertext block have 4 bits minimum (carry + message)
    fn unchecked_compare<T, F>(
        &self,
        lhs: &T,
        rhs: &T,
        sign_result_handler_fn: F,
    ) -> crate::shortint::Ciphertext
    where
        T: IntegerRadixCiphertext,
        F: Fn(u64) -> u64,
    {
        assert_eq!(lhs.blocks().len(), rhs.blocks().len());

        // false positive as compare_blocks does not mean the same in both branches
        #[allow(clippy::branches_sharing_code)]
        let compare_blocks_fn = if lhs.blocks()[0].carry_modulus.0
            < lhs.blocks()[0].message_modulus.0
        {
            fn compare_blocks(
                comparator: &Comparator,
                lhs_blocks: &[crate::shortint::Ciphertext],
                rhs_blocks: &[crate::shortint::Ciphertext],
                out_comparisons: &mut Vec<crate::shortint::Ciphertext>,
            ) {
                out_comparisons.reserve(lhs_blocks.len());
                for i in 0..lhs_blocks.len() {
                    let mut lhs = lhs_blocks[i].clone();
                    let rhs = &rhs_blocks[i];

                    comparator.compare_block_assign(&mut lhs, rhs);
                    out_comparisons.push(lhs);
                }
            }
            compare_blocks
        } else {
            fn compare_blocks(
                comparator: &Comparator,
                lhs_blocks: &[crate::shortint::Ciphertext],
                rhs_blocks: &[crate::shortint::Ciphertext],
                out_comparisons: &mut Vec<crate::shortint::Ciphertext>,
            ) {
                let identity = comparator.server_key.key.generate_lookup_table(|x| x);
                let mut lhs_chunks_iter = lhs_blocks.chunks_exact(2);
                let mut rhs_chunks_iter = rhs_blocks.chunks_exact(2);
                out_comparisons.reserve(lhs_chunks_iter.len() + lhs_chunks_iter.remainder().len());

                for (lhs_chunk, rhs_chunk) in lhs_chunks_iter.by_ref().zip(rhs_chunks_iter.by_ref())
                {
                    let mut packed_lhs = comparator.pack_block_chunk(lhs_chunk);
                    let mut packed_rhs = comparator.pack_block_chunk(rhs_chunk);
                    comparator
                        .server_key
                        .key
                        .apply_lookup_table_assign(&mut packed_lhs, &identity);
                    comparator
                        .server_key
                        .key
                        .apply_lookup_table_assign(&mut packed_rhs, &identity);
                    comparator.compare_block_assign(&mut packed_lhs, &packed_rhs);
                    out_comparisons.push(packed_lhs);
                }

                if let ([last_lhs_block], [last_rhs_block]) =
                    (lhs_chunks_iter.remainder(), rhs_chunks_iter.remainder())
                {
                    let mut last_lhs_block = last_lhs_block.clone();
                    comparator
                        .server_key
                        .key
                        .apply_lookup_table_assign(&mut last_lhs_block, &identity);
                    comparator.compare_block_assign(&mut last_lhs_block, last_rhs_block);
                    out_comparisons.push(last_lhs_block);
                }
            }

            compare_blocks
        };

        let mut comparisons = Vec::new();
        if T::IS_SIGNED {
            let (lhs_last_block, lhs_ls_blocks) = lhs.blocks().split_last().unwrap();
            let (rhs_last_block, rhs_ls_blocks) = rhs.blocks().split_last().unwrap();
            compare_blocks_fn(self, lhs_ls_blocks, rhs_ls_blocks, &mut comparisons);
            let last_block_cmp = self.compare_blocks_with_sign_bit(lhs_last_block, rhs_last_block);

            comparisons.push(last_block_cmp);
        } else {
            compare_blocks_fn(self, lhs.blocks(), rhs.blocks(), &mut comparisons);
        }

        self.reduce_signs(comparisons, sign_result_handler_fn)
    }

    /// Expects the carry buffers to be empty
    ///
    /// Requires that the RadixCiphertext block have 4 bits minimum (carry + message)
    ///
    /// This functions takes two integer ciphertext:
    ///
    /// It returns a Vec of block that will contain the sign of the comparison
    /// (Self::IS_INFERIOR, Self::IS_EQUAL, Self::IS_SUPERIOR)
    ///
    /// The output len may be shorter as blocks may be packed
    fn unchecked_compare_parallelized<T, F>(
        &self,
        lhs: &T,
        rhs: &T,
        sign_result_handler_fn: F,
    ) -> crate::shortint::Ciphertext
    where
        T: IntegerRadixCiphertext,
        F: Fn(u64) -> u64,
    {
        assert_eq!(lhs.blocks().len(), rhs.blocks().len());

        let num_block = lhs.blocks().len();

        // false positive as compare_blocks does not mean the same in both branches
        #[allow(clippy::branches_sharing_code)]
        let compare_blocks_fn =
            if lhs.blocks()[0].carry_modulus.0 < lhs.blocks()[0].message_modulus.0 {
                /// Compares blocks in parallel
                fn compare_blocks(
                    comparator: &Comparator,
                    lhs_blocks: &[crate::shortint::Ciphertext],
                    rhs_blocks: &[crate::shortint::Ciphertext],
                    out_comparisons: &mut Vec<crate::shortint::Ciphertext>,
                ) {
                    lhs_blocks
                        .par_iter()
                        .zip(rhs_blocks.par_iter())
                        .map(|(lhs, rhs)| {
                            let mut lhs = lhs.clone();
                            comparator.compare_block_assign(&mut lhs, rhs);
                            lhs
                        })
                        .collect_into_vec(out_comparisons);
                }

                compare_blocks
            } else {
                /// Compares blocks in parallel, using the fact that they can be packed
                fn compare_blocks(
                    comparator: &Comparator,
                    lhs_blocks: &[crate::shortint::Ciphertext],
                    rhs_blocks: &[crate::shortint::Ciphertext],
                    out_comparisons: &mut Vec<crate::shortint::Ciphertext>,
                ) {
                    // After packing we have to clean the noise
                    let identity = comparator.server_key.key.generate_lookup_table(|x| x);
                    lhs_blocks
                        .par_chunks(2)
                        .zip(rhs_blocks.par_chunks(2))
                        .map(|(lhs_chunk, rhs_chunk)| {
                            let (mut packed_lhs, packed_rhs) = rayon::join(
                                || {
                                    let mut block = comparator.pack_block_chunk(lhs_chunk);
                                    comparator
                                        .server_key
                                        .key
                                        .apply_lookup_table_assign(&mut block, &identity);
                                    block
                                },
                                || {
                                    let mut block = comparator.pack_block_chunk(rhs_chunk);
                                    comparator
                                        .server_key
                                        .key
                                        .apply_lookup_table_assign(&mut block, &identity);
                                    block
                                },
                            );

                            comparator.compare_block_assign(&mut packed_lhs, &packed_rhs);
                            packed_lhs
                        })
                        .collect_into_vec(out_comparisons);
                }
                compare_blocks
            };

        let mut comparisons = Vec::with_capacity(num_block);

        if T::IS_SIGNED {
            let (lhs_last_block, lhs_ls_blocks) = lhs.blocks().split_last().unwrap();
            let (rhs_last_block, rhs_ls_blocks) = rhs.blocks().split_last().unwrap();
            let (_, last_block_cmp) = rayon::join(
                || {
                    compare_blocks_fn(self, lhs_ls_blocks, rhs_ls_blocks, &mut comparisons);
                },
                || self.compare_blocks_with_sign_bit(lhs_last_block, rhs_last_block),
            );

            comparisons.push(last_block_cmp);
        } else {
            compare_blocks_fn(self, lhs.blocks(), rhs.blocks(), &mut comparisons);
        }

        self.reduce_signs_parallelized(comparisons, sign_result_handler_fn)
    }

    /// This functions takes two slices:
    ///
    /// - one of encrypted blocks
    /// - the other of scalar to compare to each encrypted block
    ///
    /// It returns a Vec of block that will contain the sign of the comparison
    /// (Self::IS_INFERIOR, Self::IS_EQUAL, Self::IS_SUPERIOR)
    ///
    /// The output len is half the input len as blocks will be packed
    fn unchecked_scalar_block_slice_compare_parallelized(
        &self,
        lhs_blocks: &[Ciphertext],
        scalar_blocks: &[u8],
    ) -> Vec<crate::shortint::Ciphertext> {
        assert_eq!(lhs_blocks.len(), scalar_blocks.len());
        let num_blocks = lhs_blocks.len();
        let num_blocks_halved = (num_blocks / 2) + (num_blocks % 2);

        let message_modulus = self.server_key.key.message_modulus.0;
        let mut signs = Vec::with_capacity(num_blocks_halved);
        lhs_blocks
            .par_chunks(2)
            .zip(scalar_blocks.par_chunks(2))
            .map(|(lhs_chunk, scalar_chunk)| {
                let packed_scalar = scalar_chunk[0]
                    + (scalar_chunk.get(1).copied().unwrap_or(0) * message_modulus as u8);
                let mut packed_lhs = self.pack_block_chunk(lhs_chunk);
                self.scalar_compare_block_assign(&mut packed_lhs, packed_scalar);
                packed_lhs
            })
            .collect_into_vec(&mut signs);

        signs
    }

    /// Computes the sign of an integer ciphertext with a clear value
    ///
    /// * The ciphertext can be unsigned or signed
    /// * The clear value can be positive or negative
    fn unchecked_scalar_compare_parallelized<T, Scalar, F>(
        &self,
        lhs: &T,
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> Ciphertext
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64,
    {
        if T::IS_SIGNED {
            match self.server_key.is_scalar_out_of_bounds(lhs, rhs) {
                Some(std::cmp::Ordering::Greater) => {
                    // Scalar is greater than the bounds, so ciphertext is smaller
                    return self
                        .server_key
                        .key
                        .create_trivial(sign_result_handler_fn(Self::IS_INFERIOR));
                }
                Some(std::cmp::Ordering::Less) => {
                    // Scalar is smaller than the bounds, so ciphertext is bigger
                    return self
                        .server_key
                        .key
                        .create_trivial(sign_result_handler_fn(Self::IS_SUPERIOR));
                }
                Some(std::cmp::Ordering::Equal) => unreachable!("Internal error: invalid value"),
                None => {
                    // scalar is in range, fallthrough
                }
            }

            if rhs >= Scalar::ZERO {
                self.signed_unchecked_scalar_compare_with_positive_scalar_parallelized(
                    lhs.blocks(),
                    rhs,
                    sign_result_handler_fn,
                )
            } else {
                let scalar_as_trivial: T = self
                    .server_key
                    .create_trivial_radix(rhs, lhs.blocks().len());
                self.unchecked_compare_parallelized(lhs, &scalar_as_trivial, sign_result_handler_fn)
            }
        } else {
            self.unsigned_unchecked_scalar_compare_blocks_parallelized(
                lhs.blocks(),
                rhs,
                sign_result_handler_fn,
            )
        }
    }

    /// This function computes the sign of a signed integer ciphertext with
    /// a positive clear value
    ///
    /// Scalar **must** be >= 0
    /// Scalar must be <= the max value possible for lhs
    fn signed_unchecked_scalar_compare_with_positive_scalar_parallelized<Scalar, F>(
        &self,
        lhs_blocks: &[Ciphertext],
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> Ciphertext
    where
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64,
    {
        assert!(!lhs_blocks.is_empty());
        assert!(rhs >= Scalar::ZERO);

        let message_modulus = self.server_key.key.message_modulus.0;

        let scalar_blocks = BlockDecomposer::with_early_stop_at_zero(rhs, message_modulus.ilog2())
            .iter_as::<u64>()
            .map(|x| x as u8)
            .take(lhs_blocks.len())
            .collect::<Vec<_>>();

        let (least_significant_blocks, most_significant_blocks) =
            lhs_blocks.split_at(scalar_blocks.len());

        match (
            least_significant_blocks.is_empty(),
            most_significant_blocks.is_empty(),
        ) {
            (false, false) => {
                // We have to handle both part of the work
                // And the sign bit is located in the most_significant_blocks
                let (lsb_sign, msb_sign) = rayon::join(
                    || {
                        let lsb_signs = self.unchecked_scalar_block_slice_compare_parallelized(
                            least_significant_blocks,
                            &scalar_blocks[..least_significant_blocks.len()],
                        );
                        self.reduce_signs_parallelized(lsb_signs, |x| x)
                    },
                    || {
                        // most_significant_blocks has the sign block, which must be handled
                        let are_all_msb_equal_to_zero = self.server_key.are_all_blocks_zero(
                            &most_significant_blocks[..most_significant_blocks.len() - 1],
                        );
                        let sign_bit_pos = self.server_key.key.message_modulus.0.ilog2() - 1;

                        // This LUT is the fusion of manu LUT.
                        // It defines the ordering given by the sign block (known scalar is >= 0)
                        // It defines the ordering given by the result of are_all_msb_blocks_zeros
                        // and finally reduces these two previous ordering to produce one final
                        // ordering for the considered blocks
                        let lut = self.server_key.key.generate_lookup_table_bivariate(
                            |sign_block, msb_are_zeros| {
                                let sign_bit_is_set = (sign_block >> sign_bit_pos) == 1;
                                let sign_block_ordering = if sign_bit_is_set {
                                    Self::IS_INFERIOR
                                } else if sign_block != 0 {
                                    Self::IS_SUPERIOR
                                } else {
                                    Self::IS_EQUAL
                                };
                                let msb_ordering = if msb_are_zeros == 1 {
                                    Self::IS_EQUAL
                                } else {
                                    Self::IS_SUPERIOR
                                };

                                reduce_two_orderings_function(
                                    sign_block_ordering << 2 | msb_ordering,
                                )
                            },
                        );
                        self.server_key.key.unchecked_apply_lookup_table_bivariate(
                            most_significant_blocks.last().as_ref().unwrap(),
                            &are_all_msb_equal_to_zero,
                            &lut,
                        )
                    },
                );

                // parallelized not needed as there are 2 blocks
                self.reduce_signs(vec![lsb_sign, msb_sign], sign_result_handler_fn)
            }
            (false, true) => {
                // This means lhs_blocks.len() == scalar_blocks.len()
                // We have to do only the regular block comparisons.
                // We again have to split in two, to handle the sign bit
                let n = least_significant_blocks.len();
                let (mut signs, sign_block_sign) = rayon::join(
                    || {
                        self.unchecked_scalar_block_slice_compare_parallelized(
                            &least_significant_blocks[..n - 1],
                            &scalar_blocks[..n - 1],
                        )
                    },
                    || {
                        let trivial_sign_block = self
                            .server_key
                            .key
                            .create_trivial(scalar_blocks[n - 1] as u64);
                        self.compare_blocks_with_sign_bit(
                            &least_significant_blocks[n - 1],
                            &trivial_sign_block,
                        )
                    },
                );

                signs.push(sign_block_sign);
                self.reduce_signs_parallelized(signs, sign_result_handler_fn)
            }
            (true, false) => {
                // We only have to compare blocks with zero
                // (means scalar is zero)
                // This also means scalar_block.len() == 0, i.e scalar == 0

                let n = most_significant_blocks.len();
                let are_all_msb_zeros = self
                    .server_key
                    .are_all_blocks_zero(&most_significant_blocks[..n - 1]);

                let sign_bit_pos = self.server_key.key.message_modulus.0.ilog2() - 1;

                let sign_block_ordering_with_respect_to_zero = |sign_block| {
                    let sign_block = sign_block % self.server_key.key.message_modulus.0 as u64;
                    let sign_bit_is_set = (sign_block >> sign_bit_pos) == 1;
                    if sign_bit_is_set {
                        Self::IS_INFERIOR
                    } else if sign_block != 0 {
                        Self::IS_SUPERIOR
                    } else {
                        Self::IS_EQUAL
                    }
                };

                // Fuse multiple LUT into one
                // Use the previously defined function to get ordering of sign block
                // Define ordering for comparison with zero
                // reduce these two ordering to get the final one
                let lut = self.server_key.key.generate_lookup_table_bivariate(
                    |are_all_zeros, sign_block| {
                        // "re-code" are_all_zeros as an ordering value
                        let are_all_zeros = if are_all_zeros == 1 {
                            Self::IS_EQUAL
                        } else {
                            Self::IS_SUPERIOR
                        };

                        let x = (sign_block_ordering_with_respect_to_zero(sign_block) << 2)
                            + are_all_zeros;
                        sign_result_handler_fn(reduce_two_orderings_function(x))
                    },
                );
                self.server_key.key.unchecked_apply_lookup_table_bivariate(
                    &are_all_msb_zeros,
                    &most_significant_blocks[n - 1],
                    &lut,
                )
            }
            (true, true) => {
                // assert should have  been hit earlier
                unreachable!("Empty input ciphertext")
            }
        }
    }

    /// This function computes the sign of a unsigned integer ciphertext
    /// with a clear value.
    ///
    /// * lhs_blocks **must** represent positive values
    /// * rhs can be positive of negative
    fn unsigned_unchecked_scalar_compare_blocks_parallelized<Scalar, F>(
        &self,
        lhs_blocks: &[Ciphertext],
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> Ciphertext
    where
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64,
    {
        assert!(!lhs_blocks.is_empty());

        if rhs < Scalar::ZERO {
            // lhs_blocks represent an unsigned (always >= 0)
            return self
                .server_key
                .key
                .create_trivial(sign_result_handler_fn(Self::IS_SUPERIOR));
        }

        let message_modulus = self.server_key.key.message_modulus.0;

        let mut scalar_blocks =
            BlockDecomposer::with_early_stop_at_zero(rhs, message_modulus.ilog2())
                .iter_as::<u64>()
                .map(|x| x as u8)
                .collect::<Vec<_>>();

        // scalar is obviously bigger if it has non-zero
        // blocks  after lhs's last block
        let is_scalar_obviously_bigger = scalar_blocks
            .get(lhs_blocks.len()..)
            .is_some_and(|sub_slice| sub_slice.iter().any(|&scalar_block| scalar_block != 0));
        if is_scalar_obviously_bigger {
            return self
                .server_key
                .key
                .create_trivial(sign_result_handler_fn(Self::IS_INFERIOR));
        }
        // If we are sill here, that means scalar_blocks above
        // num_blocks are 0s, we can remove them
        // as we will handle them separately.
        scalar_blocks.truncate(lhs_blocks.len());

        let (least_significant_blocks, most_significant_blocks) =
            lhs_blocks.split_at(scalar_blocks.len());

        // Reducing the signs is the bottleneck of the comparison algorithms,
        // however if the scalar case there is an improvement:
        //
        // The idea is to reduce the number of signs block we have to
        // reduce. We can do that by splitting the comparison problem in two parts.
        //
        // - One part where we compute the signs block between the scalar with just enough blocks
        //   from the ciphertext that can represent the scalar value
        //
        // - The other part is to compare the ciphertext blocks not considered for the sign
        //   computation with zero, and create a single sign block from that.
        //
        // The smaller the scalar value is compared to the ciphertext num bits encrypted,
        // the more the comparisons with zeros we have to do,
        // and the less signs block we will have to reduce.
        //
        // This will create a speedup as comparing a bunch of blocks with 0
        // is faster

        match (
            least_significant_blocks.is_empty(),
            most_significant_blocks.is_empty(),
        ) {
            (false, false) => {
                // We have to handle both part of the work described above

                let (lsb_sign, are_all_msb_equal_to_zero) = rayon::join(
                    || {
                        let lsb_signs = self.unchecked_scalar_block_slice_compare_parallelized(
                            least_significant_blocks,
                            &scalar_blocks,
                        );
                        self.reduce_signs_parallelized(lsb_signs, |x| x)
                    },
                    || self.server_key.are_all_blocks_zero(most_significant_blocks),
                );

                // Reduce the two blocks into one final
                let lut = self
                    .server_key
                    .key
                    .generate_lookup_table_bivariate(|lsb, msb| {
                        // "re-code" are_all_msb_equal_to_zero as an ordering value
                        let msb = if msb == 1 {
                            Self::IS_EQUAL
                        } else {
                            Self::IS_SUPERIOR
                        };

                        let x = (msb << 2) + lsb;
                        let final_sign = reduce_two_orderings_function(x);
                        sign_result_handler_fn(final_sign)
                    });

                self.server_key.key.unchecked_apply_lookup_table_bivariate(
                    &lsb_sign,
                    &are_all_msb_equal_to_zero,
                    &lut,
                )
            }
            (false, true) => {
                // We only have to do the regular comparison
                // And not the part where we compare most significant blocks with zeros
                let signs = self.unchecked_scalar_block_slice_compare_parallelized(
                    least_significant_blocks,
                    &scalar_blocks,
                );
                self.reduce_signs_parallelized(signs, sign_result_handler_fn)
            }
            (true, false) => {
                // We only have to compare blocks with zero
                // means scalar is zero
                let are_all_msb_equal_to_zero =
                    self.server_key.are_all_blocks_zero(most_significant_blocks);
                let lut = self.server_key.key.generate_lookup_table(|x| {
                    // "re-code" x as an ordering value
                    let x = if x == 1 {
                        Self::IS_EQUAL
                    } else {
                        Self::IS_SUPERIOR
                    };
                    sign_result_handler_fn(x)
                });
                self.server_key
                    .key
                    .apply_lookup_table(&are_all_msb_equal_to_zero, &lut)
            }
            (true, true) => {
                // assert should have  been hit earlier
                unreachable!("Empty input ciphertext")
            }
        }
    }

    fn smart_compare<T, F>(
        &self,
        lhs: &mut T,
        rhs: &mut T,
        sign_result_handler_fn: F,
    ) -> crate::shortint::Ciphertext
    where
        T: IntegerRadixCiphertext,
        F: Fn(u64) -> u64,
    {
        if !lhs.block_carries_are_empty() {
            self.server_key.full_propagate(lhs);
        }
        if !rhs.block_carries_are_empty() {
            self.server_key.full_propagate(rhs);
        }
        self.unchecked_compare(lhs, rhs, sign_result_handler_fn)
    }

    fn smart_compare_parallelized<T, F>(
        &self,
        lhs: &mut T,
        rhs: &mut T,
        sign_result_handler_fn: F,
    ) -> crate::shortint::Ciphertext
    where
        T: IntegerRadixCiphertext,
        F: Fn(u64) -> u64,
    {
        rayon::join(
            || {
                if !lhs.block_carries_are_empty() {
                    self.server_key.full_propagate_parallelized(lhs);
                }
            },
            || {
                if !rhs.block_carries_are_empty() {
                    self.server_key.full_propagate_parallelized(rhs);
                }
            },
        );
        self.unchecked_compare_parallelized(lhs, rhs, sign_result_handler_fn)
    }

    /// Expects the carry buffers to be empty
    fn unchecked_min_or_max<T>(&self, lhs: &T, rhs: &T, selector: MinMaxSelector) -> T
    where
        T: IntegerRadixCiphertext,
    {
        let (lhs_lut, rhs_lut) = match selector {
            MinMaxSelector::Max => (&self.lhs_lut, &self.rhs_lut),
            MinMaxSelector::Min => (&self.rhs_lut, &self.lhs_lut),
        };
        let num_block = lhs.blocks().len();

        let mut offset = self.unchecked_compare(lhs, rhs, |x| x);
        self.server_key
            .key
            .apply_lookup_table_assign(&mut offset, &self.comparison_result_to_offset_lut);

        let mut result = Vec::with_capacity(num_block);
        for i in 0..num_block {
            let lhs_block = self.server_key.key.unchecked_add(&lhs.blocks()[i], &offset);
            let rhs_block = self.server_key.key.unchecked_add(&rhs.blocks()[i], &offset);

            let maybe_lhs = self.server_key.key.apply_lookup_table(&lhs_block, lhs_lut);
            let maybe_rhs = self.server_key.key.apply_lookup_table(&rhs_block, rhs_lut);

            let r = self.server_key.key.unchecked_add(&maybe_lhs, &maybe_rhs);
            result.push(r);
        }

        T::from_blocks(result)
    }

    /// Expects the carry buffers to be empty
    fn unchecked_min_or_max_parallelized<T>(&self, lhs: &T, rhs: &T, selector: MinMaxSelector) -> T
    where
        T: IntegerRadixCiphertext,
    {
        let sign = self.unchecked_compare_parallelized(lhs, rhs, |x| x);
        let do_clean_message = true;
        match selector {
            MinMaxSelector::Max => self
                .server_key
                .unchecked_programmable_if_then_else_parallelized(
                    &sign,
                    lhs,
                    rhs,
                    |sign| sign == Self::IS_SUPERIOR,
                    do_clean_message,
                ),
            MinMaxSelector::Min => self
                .server_key
                .unchecked_programmable_if_then_else_parallelized(
                    &sign,
                    lhs,
                    rhs,
                    |sign| sign == Self::IS_INFERIOR,
                    do_clean_message,
                ),
        }
    }

    fn unchecked_scalar_min_or_max_parallelized<T, Scalar>(
        &self,
        lhs: &T,
        rhs: Scalar,
        selector: MinMaxSelector,
    ) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        let sign = self.unchecked_scalar_compare_parallelized(lhs, rhs, |x| x);
        let rhs = self
            .server_key
            .create_trivial_radix(rhs, lhs.blocks().len());
        let do_clean_message = true;
        match selector {
            MinMaxSelector::Max => self
                .server_key
                .unchecked_programmable_if_then_else_parallelized(
                    &sign,
                    lhs,
                    &rhs,
                    |sign| sign == Self::IS_SUPERIOR,
                    do_clean_message,
                ),
            MinMaxSelector::Min => self
                .server_key
                .unchecked_programmable_if_then_else_parallelized(
                    &sign,
                    lhs,
                    &rhs,
                    |sign| sign == Self::IS_INFERIOR,
                    do_clean_message,
                ),
        }
    }

    fn smart_min_or_max<T>(&self, lhs: &mut T, rhs: &mut T, selector: MinMaxSelector) -> T
    where
        T: IntegerRadixCiphertext,
    {
        if !lhs.block_carries_are_empty() {
            self.server_key.full_propagate_parallelized(lhs);
        }
        if !rhs.block_carries_are_empty() {
            self.server_key.full_propagate_parallelized(rhs);
        }
        self.unchecked_min_or_max(lhs, rhs, selector)
    }

    fn smart_min_or_max_parallelized<T>(
        &self,
        lhs: &mut T,
        rhs: &mut T,
        selector: MinMaxSelector,
    ) -> T
    where
        T: IntegerRadixCiphertext,
    {
        rayon::join(
            || {
                if !lhs.block_carries_are_empty() {
                    self.server_key.full_propagate_parallelized(lhs);
                }
            },
            || {
                if !rhs.block_carries_are_empty() {
                    self.server_key.full_propagate_parallelized(rhs);
                }
            },
        );
        self.unchecked_min_or_max_parallelized(lhs, rhs, selector)
    }

    //======================================
    // Unchecked Single-Threaded operations
    //======================================

    pub fn unchecked_gt<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_SUPERIOR);
        let comparison = self.unchecked_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_ge<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_SUPERIOR);
        let comparison = self.unchecked_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_lt<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_INFERIOR);
        let comparison = self.unchecked_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_le<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_INFERIOR);
        let comparison = self.unchecked_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_max<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.unchecked_min_or_max(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn unchecked_min<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.unchecked_min_or_max(lhs, rhs, MinMaxSelector::Min)
    }

    //======================================
    // Unchecked Multi-Threaded operations
    //======================================

    pub fn unchecked_gt_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_SUPERIOR);
        let comparison = self.unchecked_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_ge_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_SUPERIOR);
        let comparison = self.unchecked_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_lt_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_INFERIOR);
        let comparison = self.unchecked_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_le_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_INFERIOR);
        let comparison = self.unchecked_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_max_parallelized<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.unchecked_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn unchecked_min_parallelized<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.unchecked_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Min)
    }

    //======================================
    // Smart Single-Threaded operations
    //======================================

    pub fn smart_gt<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_SUPERIOR);
        let comparison = self.smart_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_ge<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_SUPERIOR);
        let comparison = self.smart_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_lt<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_INFERIOR);
        let comparison = self.smart_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_le<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_INFERIOR);
        let comparison = self.smart_compare(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_max<T>(&self, lhs: &mut T, rhs: &mut T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.smart_min_or_max(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn smart_min<T>(&self, lhs: &mut T, rhs: &mut T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.smart_min_or_max(lhs, rhs, MinMaxSelector::Min)
    }

    //======================================
    // Smart Multi-Threaded operations
    //======================================

    pub fn smart_gt_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_SUPERIOR);
        let comparison = self.smart_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_ge_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_SUPERIOR);
        let comparison = self.smart_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_lt_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_INFERIOR);
        let comparison = self.smart_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_le_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let sign_result_handler_fn = |x| u64::from(x == Self::IS_EQUAL || x == Self::IS_INFERIOR);
        let comparison = self.smart_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn smart_max_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.smart_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn smart_min_parallelized<T>(&self, lhs: &mut T, rhs: &mut T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        self.smart_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Min)
    }

    //======================================
    // "Default" Multi-Threaded operations
    //======================================

    pub fn gt_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;
        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_gt_parallelized(lhs, rhs)
    }

    pub fn ge_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;
        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_ge_parallelized(lhs, rhs)
    }

    pub fn lt_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;
        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_lt_parallelized(lhs, rhs)
    }

    pub fn le_parallelized<T>(&self, lhs: &T, rhs: &T) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;
        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_le_parallelized(lhs, rhs)
    }

    pub fn max_parallelized<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;

        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_max_parallelized(lhs, rhs)
    }

    pub fn min_parallelized<T>(&self, lhs: &T, rhs: &T) -> T
    where
        T: IntegerRadixCiphertext,
    {
        let mut tmp_lhs;
        let mut tmp_rhs;

        let (lhs, rhs) = match (lhs.block_carries_are_empty(), rhs.block_carries_are_empty()) {
            (true, true) => (lhs, rhs),
            (true, false) => {
                tmp_rhs = rhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_rhs);
                (lhs, &tmp_rhs)
            }
            (false, true) => {
                tmp_lhs = lhs.clone();
                self.server_key.full_propagate_parallelized(&mut tmp_lhs);
                (&tmp_lhs, rhs)
            }
            (false, false) => {
                tmp_lhs = lhs.clone();
                tmp_rhs = rhs.clone();
                rayon::join(
                    || self.server_key.full_propagate_parallelized(&mut tmp_lhs),
                    || self.server_key.full_propagate_parallelized(&mut tmp_rhs),
                );
                (&tmp_lhs, &tmp_rhs)
            }
        };

        self.unchecked_min_parallelized(lhs, rhs)
    }

    //===========================================
    // Unchecked Scalar Multi-Threaded operations
    //===========================================

    /// This functions calls the unchecked comparison function
    /// which returns whether lhs is inferior, equal or greater than rhs,
    /// and maps the result to a homomorphic bool value (0 or 1) using the provided function.
    pub fn unchecked_scalar_compare_parallelized_handler<T, Scalar, F>(
        &self,
        lhs: &T,
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64 + Sync,
    {
        let comparison =
            self.unchecked_scalar_compare_parallelized(lhs, rhs, sign_result_handler_fn);
        BooleanBlock::new_unchecked(comparison)
    }

    pub fn unchecked_scalar_gt_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, |x| {
            u64::from(x == Self::IS_SUPERIOR)
        })
    }

    pub fn unchecked_scalar_ge_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, |x| {
            u64::from(x == Self::IS_SUPERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn unchecked_scalar_lt_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, |x| {
            u64::from(x == Self::IS_INFERIOR)
        })
    }

    pub fn unchecked_scalar_le_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, |x| {
            u64::from(x == Self::IS_INFERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn unchecked_scalar_max_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn unchecked_scalar_min_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Min)
    }

    //=======================================
    // Smart Scalar Multi-Threaded operations
    //=======================================

    fn smart_scalar_compare_parallelized<T, Scalar, F>(
        &self,
        lhs: &mut T,
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64 + Sync,
    {
        if !lhs.block_carries_are_empty() {
            self.server_key.full_propagate_parallelized(lhs);
        }
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, sign_result_handler_fn)
    }

    pub fn smart_scalar_gt_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.smart_scalar_compare_parallelized(lhs, rhs, |x| u64::from(x == Self::IS_SUPERIOR))
    }

    pub fn smart_scalar_ge_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.smart_scalar_compare_parallelized(lhs, rhs, |x| {
            u64::from(x == Self::IS_SUPERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn smart_scalar_lt_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.smart_scalar_compare_parallelized(lhs, rhs, |x| u64::from(x == Self::IS_INFERIOR))
    }

    pub fn smart_scalar_le_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.smart_scalar_compare_parallelized(lhs, rhs, |x| {
            u64::from(x == Self::IS_INFERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn smart_scalar_max_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        if !lhs.block_carries_are_empty() {
            self.server_key.full_propagate_parallelized(lhs);
        }
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn smart_scalar_min_parallelized<T, Scalar>(&self, lhs: &mut T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        if !lhs.block_carries_are_empty() {
            self.server_key.full_propagate_parallelized(lhs);
        }
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Min)
    }

    //======================================
    // "Default" Scalar Multi-Threaded operations
    //======================================

    fn default_scalar_compare_parallelized<T, Scalar, F>(
        &self,
        lhs: &T,
        rhs: Scalar,
        sign_result_handler_fn: F,
    ) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
        F: Fn(u64) -> u64 + Sync,
    {
        let mut tmp_lhs;
        let lhs = if lhs.block_carries_are_empty() {
            lhs
        } else {
            tmp_lhs = lhs.clone();
            self.server_key.full_propagate_parallelized(&mut tmp_lhs);
            &tmp_lhs
        };
        self.unchecked_scalar_compare_parallelized_handler(lhs, rhs, sign_result_handler_fn)
    }

    pub fn scalar_gt_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.default_scalar_compare_parallelized(lhs, rhs, |x| u64::from(x == Self::IS_SUPERIOR))
    }

    pub fn scalar_ge_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.default_scalar_compare_parallelized(lhs, rhs, |x| {
            u64::from(x == Self::IS_SUPERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn scalar_lt_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.default_scalar_compare_parallelized(lhs, rhs, |x| u64::from(x == Self::IS_INFERIOR))
    }

    pub fn scalar_le_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> BooleanBlock
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        self.default_scalar_compare_parallelized(lhs, rhs, |x| {
            u64::from(x == Self::IS_INFERIOR || x == Self::IS_EQUAL)
        })
    }

    pub fn scalar_max_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        let mut tmp_lhs;
        let lhs = if lhs.block_carries_are_empty() {
            lhs
        } else {
            tmp_lhs = lhs.clone();
            self.server_key.full_propagate_parallelized(&mut tmp_lhs);
            &tmp_lhs
        };
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Max)
    }

    pub fn scalar_min_parallelized<T, Scalar>(&self, lhs: &T, rhs: Scalar) -> T
    where
        T: IntegerRadixCiphertext,
        Scalar: DecomposableInto<u64>,
    {
        let mut tmp_lhs;
        let lhs = if lhs.block_carries_are_empty() {
            lhs
        } else {
            tmp_lhs = lhs.clone();
            self.server_key.full_propagate_parallelized(&mut tmp_lhs);
            &tmp_lhs
        };
        self.unchecked_scalar_min_or_max_parallelized(lhs, rhs, MinMaxSelector::Min)
    }
}
