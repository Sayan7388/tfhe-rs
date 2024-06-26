use crate::shortint::parameters::classic::compact_pk::gaussian::p_fail_2_minus_64::{
    ks_pbs, pbs_ks,
};
use crate::shortint::ClassicPBSParameters;
pub use tuniform::p_fail_2_minus_64::ks_pbs::*;

pub mod gaussian;
pub mod tuniform;

pub const ALL_PARAMETER_VEC_COMPACT_PK: [ClassicPBSParameters; 32] = [
    PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_3_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_4_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_5_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_6_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_7_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_3_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_4_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_5_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_2_CARRY_6_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_3_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_3_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_3_CARRY_4_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_3_CARRY_5_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_4_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_4_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_4_CARRY_3_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_5_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_5_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_5_CARRY_3_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_6_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_6_CARRY_2_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_7_CARRY_1_COMPACT_PK_KS_PBS,
    PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_PBS_KS,
    PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_PBS_KS,
    PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_PBS_KS,
    PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_PBS_KS,
];

// Aliases, to be deprecated in subsequent versions once we e.g. have the "parameter builder"
pub const PARAM_MESSAGE_1_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_3_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_3_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_4_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_4_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_5_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_5_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_6_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_6_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_7_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_1_CARRY_7_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_3_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_3_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_4_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_4_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_5_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_5_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_6_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_2_CARRY_6_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_4_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_4_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_5_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_3_CARRY_5_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_4_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_4_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_4_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_3_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_4_CARRY_3_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_5_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_5_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_5_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_5_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_5_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_5_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_5_CARRY_3_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_5_CARRY_3_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_6_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_6_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_6_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_6_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_6_CARRY_2_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_6_CARRY_2_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_7_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_7_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_7_CARRY_1_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_7_CARRY_1_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_8_CARRY_0_COMPACT_PK_KS_PBS: ClassicPBSParameters =
    ks_pbs::PARAM_MESSAGE_8_CARRY_0_COMPACT_PK_KS_PBS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_PBS_KS: ClassicPBSParameters =
    pbs_ks::PARAM_MESSAGE_1_CARRY_1_COMPACT_PK_PBS_KS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_PBS_KS: ClassicPBSParameters =
    pbs_ks::PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_PBS_KS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_PBS_KS: ClassicPBSParameters =
    pbs_ks::PARAM_MESSAGE_3_CARRY_3_COMPACT_PK_PBS_KS_GAUSSIAN_2M64;
pub const PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_PBS_KS: ClassicPBSParameters =
    pbs_ks::PARAM_MESSAGE_4_CARRY_4_COMPACT_PK_PBS_KS_GAUSSIAN_2M64;

pub const DEFAULT_COMPACT_PK: ClassicPBSParameters = PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS;
