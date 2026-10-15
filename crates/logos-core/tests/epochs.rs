//! Intervalos comparados com enumeração independente e limites u64.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_core::{EpochError, EpochRange};

#[test]
fn inclusive_epoch_window_and_full_u64_boundaries() {
    let range = EpochRange::new(3, 5).unwrap();
    assert_eq!((range.first(), range.last()), (3, 5));
    for epoch in [3, 4, 5] {
        assert!(range.contains(epoch));
    }
    for epoch in [0, 2, 6, u64::MAX] {
        assert!(!range.contains(epoch));
    }
    let full = EpochRange::new(0, u64::MAX).unwrap();
    for epoch in [0, 1, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        assert!(full.contains(epoch));
        let single = EpochRange::new(epoch, epoch).unwrap();
        assert!(single.contains(epoch));
        if epoch > 0 {
            assert!(!single.contains(epoch - 1));
        }
        if epoch < u64::MAX {
            assert!(!single.contains(epoch + 1));
        }
    }
    assert_eq!(EpochRange::new(6, 3), Err(EpochError { first: 6, last: 3 }));
    assert!(EpochRange::new(u64::MAX, 0).is_err());
}

#[test]
fn every_small_interval_matches_explicit_epoch_enumeration() {
    for first in 0..=8 {
        for last in 0..=8 {
            let expected: Vec<_> = (first..=last).collect();
            match EpochRange::new(first, last) {
                Ok(range) => {
                    assert!(!expected.is_empty());
                    for epoch in 0..=9 {
                        assert_eq!(range.contains(epoch), expected.contains(&epoch));
                    }
                }
                Err(error) => {
                    assert!(expected.is_empty());
                    assert_eq!((error.first, error.last), (first, last));
                }
            }
        }
    }
}
