use super::cmux::variance_cmux;

pub const FFT_SCALING_WEIGHT: f64 = -2.577_224_94;

/// Final reduced noise generated by the final bootstrap step.
/// Note that it does not depends from input noise, assuming the bootstrap is successful
pub fn variance_blind_rotate(
    in_lwe_dimension: u64,
    out_glwe_dimension: u64,
    out_polynomial_size: u64,
    log2_base: u64,
    level: u64,
    ciphertext_modulus_log: u32,
    variance_bsk: f64,
) -> f64 {
    in_lwe_dimension as f64
        * variance_cmux(
            out_glwe_dimension,
            out_polynomial_size,
            log2_base,
            level,
            ciphertext_modulus_log,
            variance_bsk,
        )
}

#[cfg(test)]
mod tests {
    use crate::gaussian_noise::conversion::variance_to_modular_variance;
    use concrete_security_curves::gaussian::security::minimal_variance_glwe;

    use super::*;

    #[test]
    fn security_variance_bootstrap_1() {
        let ref_modular_variance = 4.078_296_369_990_673e31;

        let polynomial_size = 1 << 12;
        let glwe_dimension = 2;

        let ciphertext_modulus_log = 64;
        let security = 128;
        let variance_bsk = minimal_variance_glwe(
            glwe_dimension,
            polynomial_size,
            ciphertext_modulus_log,
            security,
        );

        let actual = variance_blind_rotate(
            2048,
            glwe_dimension,
            polynomial_size,
            24,
            2,
            ciphertext_modulus_log,
            variance_bsk,
        );

        approx::assert_relative_eq!(
            variance_to_modular_variance(actual, ciphertext_modulus_log),
            ref_modular_variance,
            max_relative = 1e-8
        );
    }

    #[test]
    fn golden_python_prototype_security_variance_bootstrap_2() {
        // golden value include fft correction
        let golden_modular_variance = 3.269_722_907_894_341e55;

        let polynomial_size = 1 << 12;
        let glwe_dimension = 4;

        let ciphertext_modulus_log = 128;
        let security = 128;
        let variance_bsk = minimal_variance_glwe(
            glwe_dimension,
            polynomial_size,
            ciphertext_modulus_log,
            security,
        );

        let actual = variance_blind_rotate(
            1024,
            glwe_dimension,
            polynomial_size,
            5,
            9,
            ciphertext_modulus_log,
            variance_bsk,
        );

        approx::assert_relative_eq!(
            variance_to_modular_variance(actual, ciphertext_modulus_log),
            golden_modular_variance,
            max_relative = 1e-8
        );
    }
}
