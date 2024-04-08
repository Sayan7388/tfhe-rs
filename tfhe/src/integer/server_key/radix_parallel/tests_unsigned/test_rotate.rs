use crate::integer::server_key::radix_parallel::tests_cases_unsigned::{
    default_rotate_left_test, default_rotate_right_test, unchecked_rotate_left_test,
    unchecked_rotate_right_test,
};
use crate::integer::server_key::radix_parallel::tests_unsigned::CpuFunctionExecutor;
use crate::integer::tests::create_parametrized_test;
use crate::integer::ServerKey;
#[cfg(tarpaulin)]
use crate::shortint::parameters::coverage_parameters::*;
use crate::shortint::parameters::*;
use crate::shortint::parameters::classic::compact_pk::tuniform::p_fail_2_minus_40::ks_pbs::*;

create_parametrized_test!(
    integer_unchecked_rotate_right {
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 4 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

create_parametrized_test!(
    integer_unchecked_rotate_left {
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 4 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

create_parametrized_test!(
    integer_rotate_right {
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 4 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

create_parametrized_test!(
    integer_rotate_left {
        coverage => {
            COVERAGE_PARAM_MESSAGE_2_CARRY_2_KS_PBS,
            COVERAGE_PARAM_MULTI_BIT_MESSAGE_2_CARRY_2_GROUP_2_KS_PBS,
        },
        no_coverage => {
            // Requires 4 bits, so 1_1 parameters are not supported
            // until they get their own version of the algorithm
PARAM_MESSAGE_2_CARRY_2_COMPACT_PK_KS_PBS_TUNIFORM_2M40
        }
    }
);

fn integer_unchecked_rotate_right<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::unchecked_rotate_right_parallelized);
    unchecked_rotate_right_test(param, executor);
}

fn integer_rotate_right<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::rotate_right_parallelized);
    default_rotate_right_test(param, executor);
}

fn integer_unchecked_rotate_left<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::unchecked_rotate_left_parallelized);
    unchecked_rotate_left_test(param, executor);
}

fn integer_rotate_left<P>(param: P)
where
    P: Into<PBSParameters> + Copy,
{
    let executor = CpuFunctionExecutor::new(&ServerKey::rotate_left_parallelized);
    default_rotate_left_test(param, executor);
}
