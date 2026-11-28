//! Exercise the package-private adapter and its public command against the real editions.
include!("../src/bin/validate-learning-map.rs");

mod curriculum;
mod edition;
mod mechanics_tests {
    include!("mechanics.rs");
}
