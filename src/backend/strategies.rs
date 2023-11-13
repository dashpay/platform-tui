//! Strategies management backend module.

use dpp::version::PlatformVersion;
use rand::{rngs::StdRng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{transitions::create_identities_state_transitions, Strategy};

pub(crate) fn set_start_identities(strategy: &mut Strategy, count: u16, key_count: u32) {
    let identities = create_identities_state_transitions(
        count,
        key_count,
        &mut SimpleSigner::default(),
        &mut StdRng::seed_from_u64(567),
        PlatformVersion::latest(),
    );

    strategy.start_identities = identities;
}
