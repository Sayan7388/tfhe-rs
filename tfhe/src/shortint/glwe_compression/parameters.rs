use crate::shortint::parameters::{
    DecompositionBaseLog, DecompositionLevelCount, DynamicDistribution, GlweDimension,
    LweDimension, PolynomialSize, StandardDev,
};
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompressionParameters {
    pub br_level: DecompositionLevelCount,
    pub br_base_log: DecompositionBaseLog,
    pub packing_ks_level: DecompositionLevelCount,
    pub packing_ks_base_log: DecompositionBaseLog,
    pub packing_ks_polynomial_size: PolynomialSize,
    pub packing_ks_glwe_dimension: GlweDimension,
    pub lwe_per_glwe: usize,
    pub packing_ks_key_noise_distribution: DynamicDistribution<u64>,
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PostPbsParmeters {
    pub br_glwe_dimension: GlweDimension,
    pub br_polynomial_size: PolynomialSize,
    pub lwe_dimension: LweDimension,
    pub ks_key_noise_distribution: DynamicDistribution<u64>,
    pub ks_base_log: DecompositionBaseLog,
    pub ks_level: DecompositionLevelCount,
    pub pbs_base_log: DecompositionBaseLog,
    pub pbs_level: DecompositionLevelCount,
    pub br_key_noise_distribution: DynamicDistribution<u64>,
}

pub const COMP_PARAMS_FOR_MESSAGE_2_CARRY_2_KS_PBS: CompressionParameters = CompressionParameters {
    br_level: DecompositionLevelCount(1),
    br_base_log: DecompositionBaseLog(25),
    packing_ks_level: DecompositionLevelCount(2),
    packing_ks_base_log: DecompositionBaseLog(8),
    packing_ks_polynomial_size: PolynomialSize(256),
    packing_ks_glwe_dimension: GlweDimension(5),
    lwe_per_glwe: 256,
    packing_ks_key_noise_distribution: DynamicDistribution::new_gaussian_from_std_dev(StandardDev(
        8.01851773016495e-10,
    )),
};
