use crate::core_crypto::prelude::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PackedIntegers<Scalar: UnsignedInteger> {
    pub(super) packed_coeffs: Vec<Scalar>,
    pub(super) log_modulus: CiphertextModulusLog,
    pub(super) initial_len: usize,
}

impl<Scalar: UnsignedInteger> PackedIntegers<Scalar> {
    pub fn pack(slice: &[Scalar], log_modulus: CiphertextModulusLog) -> Self {
        let log_modulus = log_modulus.0;

        let in_len = slice.len();

        let number_bits_to_pack = in_len * log_modulus;

        let len = number_bits_to_pack.div_ceil(Scalar::BITS);

        // Lowest bits are on the right
        //
        // Target mapping:
        //                          log_modulus
        //                           |-------|
        //
        // slice        :    |  k+2  |  k+1  |   k   |
        // packed_coeffs:  i+1   |       i       |     i-1
        //
        //                       |---------------|
        //                         Scalar::BITS
        //
        //                                       |---|
        //                                    start_shift
        //
        //                                   |---|
        //                                   shift1
        //                             (1st loop iteration)
        //
        //                           |-----------|
        //                               shift2
        //                        (2nd loop iteration)
        //
        // packed_coeffs[i] =
        //                    slice[k] >> start_shift
        //                  | slice[k+1] << shift1
        //                  | slice[k+2] << shift2
        //
        // In the lowest bits of packed_coeffs[i], we want the highest bits of slice[k],
        // hence the right shift
        // The next bits should be the bits of slice[k+1] which we must left shifted to avoid
        // overlapping
        // This goes on
        let packed_coeffs = (0..len)
            .map(|i| {
                let k = Scalar::BITS * i / log_modulus;
                let mut j = k;

                let start_shift = i * Scalar::BITS - j * log_modulus;

                debug_assert_eq!(slice[j] >> log_modulus, Scalar::ZERO);

                let mut value = slice[j] >> start_shift;
                j += 1;

                while j * log_modulus < ((i + 1) * Scalar::BITS) && j < slice.len() {
                    let shift = j * log_modulus - i * Scalar::BITS;

                    debug_assert_eq!(slice[j] >> log_modulus, Scalar::ZERO);

                    value |= slice[j] << shift;

                    j += 1;
                }
                value
            })
            .collect();

        let log_modulus = CiphertextModulusLog(log_modulus);

        Self {
            packed_coeffs,
            log_modulus,
            initial_len: slice.len(),
        }
    }

    pub fn unpack(&self) -> impl Iterator<Item = Scalar> + '_ {
        let log_modulus = self.log_modulus.0;

        // log_modulus lowest bits set to 1
        let mask = (Scalar::ONE << log_modulus) - Scalar::ONE;

        (0..self.initial_len).map(move |i| {
            let start = i * log_modulus;
            let end = (i + 1) * log_modulus;

            let start_block = start / Scalar::BITS;
            let start_remainder = start % Scalar::BITS;

            let end_block_inclusive = (end - 1) / Scalar::BITS;

            if start_block == end_block_inclusive {
                // Lowest bits are on the right
                //
                // Target mapping:
                //                                   Scalar::BITS
                //                                |---------------|
                //
                // packed_coeffs: | start_block+1 |  start_block  |
                // container    :             |  i+1  |   i   |  i-1  |
                //
                //                                    |-------|
                //                                   log_modulus
                //
                //                                            |---|
                //                                       start_remainder
                //
                // In container[i] we want the bits of packed_coeffs[start_block] starting from
                // index start_remainder
                //
                // container[i] = lowest_bits of single_part
                //
                let single_part = self.packed_coeffs[start_block] >> start_remainder;

                single_part & mask
            } else {
                // Lowest bits are on the right
                //
                // Target mapping:
                //                                   Scalar::BITS
                //                                 |---------------|
                //
                // packed_coeffs:  | start_block+1 |  start_block  |
                // container    :      |  i+1  |   i   |  i-1  |
                //
                //                             |-------|
                //                            log_modulus
                //
                //                                     |-----------|
                //                                    start_remainder
                //
                //                                 |---|
                //                     Scalar::BITS - start_remainder
                //
                // In the lowest bits of container[i] we want the highest bits of
                // packed_coeffs[start_block] starting from index start_remainder
                //
                // In the next bits, we want the lowest bits of packed_coeffs[start_block + 1]
                // left shifted to avoid overlapping
                //
                // container[i] = lowest_bits of (first_part|second_part)
                //
                assert_eq!(end_block_inclusive, start_block + 1);

                let first_part = self.packed_coeffs[start_block] >> start_remainder;

                let second_part =
                    self.packed_coeffs[start_block + 1] << (Scalar::BITS - start_remainder);

                (first_part | second_part) & mask
            }
        })
    }
}
