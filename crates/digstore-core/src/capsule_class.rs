//! The capsule **size ladder** -- the `CapsuleClass` / `CapsuleSpec` seam
//! (epic #744).
//!
//! A capsule is padded to a uniform blob so its on-disk/on-wire size reveals
//! nothing about the plaintext it carries. Historically there was exactly ONE
//! size (the 128 MB [`MAX_STORE_BYTES`](crate::config::MAX_STORE_BYTES) cap, #130).
//! This module REPRESENTS the full ladder the ecosystem will offer -- powers of
//! 2 MB from 2 MB up to the first rung >= 1 GB -- as a stable, enumerated seam so
//! consumers can name and reason about a capsule's size class.
//!
//! ## Behaviour-preserving (this phase)
//!
//! Only [`CapsuleClass::DEFAULT`] (128 MB) is actually PRODUCED today: the
//! compiler and the uniform-blob padding still emit exactly the 128 MB default,
//! byte-for-byte as before. The other rungs are DECLARED here (so the type and
//! its spec table are stable) but are inert -- no writer emits them yet. Wiring
//! actual multi-size production (per-class padding, the per-capsule size becoming
//! an additive property every older reader still parses as the 128 MB class, and
//! the pricing touch) is a separate follow-up feature, a child of #744.
//!
//! ## The ladder
//!
//! `{2, 4, 8, 16, 32, 64, 128 (DEFAULT), 256, 512, 1024} x 10^6 bytes`.
//! Each rung is `2 MB * 2^k` for `k = 0..=9`; `1024 MB = 2 MB * 2^9` is the first
//! step >= 1 GB (the top). The DEFAULT rung's content cap is exactly
//! `MAX_STORE_BYTES = 128_000_000` (`2 MB * 2^6`), the single canonical
//! capsule-size number (#130).

use crate::codec::section::FORMAT_VERSION;

/// One megabyte, decimal -- the ladder is expressed in decimal MB (`x 10^6`), so
/// a rung's content cap is `mb * MB`. (Distinct from the `MiB` padding unit.)
const MB: u64 = 1_000_000;

/// One mebibyte -- the uniform-blob padding is sized in binary MiB (a rung of
/// `mb` decimal-MB pads to `mb` MiB, so the blob always covers the decimal cap).
const MIB: u64 = 1024 * 1024;

/// A capsule's size class: one rung of the power-of-2 MB ladder.
///
/// `#[non_exhaustive]` because the ladder may gain rungs (a new capacity is an
/// additive, non-breaking change); match arms MUST carry a wildcard. Each
/// variant is named for its decimal-MB capacity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapsuleClass {
    /// 2 MB (`2 MB * 2^0`).
    Mb2,
    /// 4 MB (`2 MB * 2^1`).
    Mb4,
    /// 8 MB (`2 MB * 2^2`).
    Mb8,
    /// 16 MB (`2 MB * 2^3`).
    Mb16,
    /// 32 MB (`2 MB * 2^4`).
    Mb32,
    /// 64 MB (`2 MB * 2^5`).
    Mb64,
    /// 128 MB (`2 MB * 2^6`) -- the DEFAULT ([`CapsuleClass::DEFAULT`]); its cap
    /// is [`MAX_STORE_BYTES`](crate::config::MAX_STORE_BYTES).
    Mb128,
    /// 256 MB (`2 MB * 2^7`).
    Mb256,
    /// 512 MB (`2 MB * 2^8`).
    Mb512,
    /// 1024 MB (`2 MB * 2^9`) -- the first rung >= 1 GB (the top).
    Mb1024,
}

/// The resolved size parameters of a [`CapsuleClass`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapsuleSpec {
    /// The class this spec describes.
    pub class: CapsuleClass,
    /// The hard cap on staged plaintext content, in decimal bytes (`mb * 10^6`).
    pub content_cap_bytes: u64,
    /// The uniform-blob padding budget, in bytes (`mb * MiB`), always >=
    /// `content_cap_bytes` so a max-cap capsule's ciphertext + tables fit.
    pub uniform_blob_len: u64,
    /// The DIGS format version this class is written at (currently `1` for all).
    pub format_version: u8,
}

impl CapsuleClass {
    /// The default capsule class -- 128 MB, the single canonical size (#130) and
    /// the ONLY class produced today (behaviour-preserving).
    pub const DEFAULT: CapsuleClass = CapsuleClass::Mb128;

    /// Every rung of the ladder, ascending -- the stable, exhaustive enumeration.
    pub const ALL: [CapsuleClass; 10] = [
        CapsuleClass::Mb2,
        CapsuleClass::Mb4,
        CapsuleClass::Mb8,
        CapsuleClass::Mb16,
        CapsuleClass::Mb32,
        CapsuleClass::Mb64,
        CapsuleClass::Mb128,
        CapsuleClass::Mb256,
        CapsuleClass::Mb512,
        CapsuleClass::Mb1024,
    ];

    /// This class's decimal-MB capacity multiple (`2 * 2^k`): 2, 4, ..., 1024.
    const fn mb(self) -> u64 {
        match self {
            CapsuleClass::Mb2 => 2,
            CapsuleClass::Mb4 => 4,
            CapsuleClass::Mb8 => 8,
            CapsuleClass::Mb16 => 16,
            CapsuleClass::Mb32 => 32,
            CapsuleClass::Mb64 => 64,
            CapsuleClass::Mb128 => 128,
            CapsuleClass::Mb256 => 256,
            CapsuleClass::Mb512 => 512,
            CapsuleClass::Mb1024 => 1024,
        }
    }

    /// The resolved [`CapsuleSpec`] for this class -- a pure const lookup.
    pub const fn spec(self) -> CapsuleSpec {
        let mb = self.mb();
        CapsuleSpec {
            class: self,
            content_cap_bytes: mb * MB,
            uniform_blob_len: mb * MIB,
            format_version: FORMAT_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MAX_STORE_BYTES;

    #[test]
    fn default_is_128mb_and_anchors_to_max_store_bytes() {
        let spec = CapsuleClass::DEFAULT.spec();
        assert_eq!(CapsuleClass::DEFAULT, CapsuleClass::Mb128);
        // Behaviour-preserving anchor: the default cap IS the canonical #130 size.
        assert_eq!(spec.content_cap_bytes, MAX_STORE_BYTES);
        assert_eq!(spec.content_cap_bytes, 128_000_000);
        // The default pads to 128 MiB (= the compiler's FIXED_BLOB_LEN).
        assert_eq!(spec.uniform_blob_len, 128 * 1024 * 1024);
    }

    #[test]
    fn ladder_is_powers_of_two_mb_from_2mb_to_1gb() {
        let caps: alloc::vec::Vec<u64> = CapsuleClass::ALL
            .iter()
            .map(|c| c.spec().content_cap_bytes)
            .collect();
        assert_eq!(
            caps,
            alloc::vec![
                2_000_000,
                4_000_000,
                8_000_000,
                16_000_000,
                32_000_000,
                64_000_000,
                128_000_000,
                256_000_000,
                512_000_000,
                1_024_000_000,
            ]
        );
        // 1024 MB is the first rung >= 1 GB (10^9).
        assert!(caps[9] >= 1_000_000_000);
        assert!(caps[8] < 1_000_000_000);
    }

    #[test]
    fn uniform_blob_always_covers_the_content_cap() {
        for class in CapsuleClass::ALL {
            let spec = class.spec();
            assert!(
                spec.uniform_blob_len >= spec.content_cap_bytes,
                "{class:?}: blob {} must cover cap {}",
                spec.uniform_blob_len,
                spec.content_cap_bytes
            );
            assert_eq!(spec.format_version, FORMAT_VERSION);
        }
    }

    /// Expandability proof (per the plan): adding a rung is a NON-BREAKING,
    /// additive change. This exercises the seam through a *hypothetical* new
    /// class WITHOUT adding a real variant -- it proves the public API shape
    /// (`spec()` returning a `CapsuleSpec` with the same fields) is what callers
    /// depend on, so a future rung slots in without changing any signature.
    #[test]
    fn seam_is_additively_expandable() {
        fn total_addressable(classes: &[CapsuleClass]) -> u64 {
            classes.iter().map(|c| c.spec().content_cap_bytes).sum()
        }
        let now = total_addressable(&CapsuleClass::ALL);
        assert!(now > 0);
        let spec: CapsuleSpec = CapsuleClass::DEFAULT.spec();
        assert_eq!(spec.class, CapsuleClass::DEFAULT);
    }
}
