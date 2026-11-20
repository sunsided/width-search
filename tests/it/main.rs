//! Integration tests for width-search, compiled as a single test binary.
//!
//! - [`novelty_properties`]: the novelty table against a naive tuple-set oracle.
//! - [`fdr_gripper`]: a hand-encoded Fast Downward task.
//! - [`fdr_translated`]: Fast Downward translator output.
//! - [`h_add`]: `h_add` values pinned on fixtures, dead ends and derived defaults.
//! - [`lapkt_counts`]: node counts on that output, pinned next to LAPKT's.
//! - [`sas_random`]: fixed seeds of random SAS+ tasks, checked against `h*`.
//! - [`search_properties`]: property tests over random SAS+ tasks.
//!
//! The SAS+ modules need the `sas-plus` feature, which provides the adapter.

#[cfg(feature = "sas-plus")]
mod fdr_gripper;
#[cfg(feature = "sas-plus")]
mod fdr_translated;
#[cfg(feature = "sas-plus")]
mod h_add;
#[cfg(feature = "sas-plus")]
mod lapkt_counts;
mod novelty_properties;
#[cfg(feature = "sas-plus")]
mod sas_random;
#[cfg(feature = "sas-plus")]
mod search_properties;
