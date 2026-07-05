//! L3 substrate adapters. Each implements `substrate::ConfidentialSubstrate` for one confidential
//! substrate; L1 stays agnostic above the trait. Re-exported at the crate root for stable paths
//! (`aperture_core::token2022`, `aperture_core::cspl`).

pub mod cspl;
pub mod token2022;
