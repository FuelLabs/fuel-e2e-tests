#[cfg(all(feature = "fuels_lts_70", feature = "fuels_71"))]
compile_error!("Features 'fuels_lts_70' and 'fuels_71' cannot be enabled at the same time.");

#[cfg(not(any(feature = "fuels_lts_70", feature = "fuels_71")))]
compile_error!("You must enable exactly one of the features: 'fuels_lts_70' or 'fuels_71'.");

pub mod helpers;
pub mod setup;

#[macro_export]
macro_rules! define_fuels {
    () => {
        // Needs to be done via extern crate because the abigen macro expects a crate "::fuels"
        // present
        #[cfg(feature = "fuels_71")]
        extern crate fuels_71 as fuels;
        #[cfg(feature = "fuels_lts_70")]
        extern crate fuels_lts_70 as fuels;
    };
}

define_fuels!();
