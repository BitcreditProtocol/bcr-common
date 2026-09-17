// ----- standard library imports
// ----- extra library imports
// ----- local imports
use crate::{BillDate, ecash};

// ----- end imports

pub const SECS_PER_DAY: u64 = 86_400;

/// Floor a unix-seconds timestamp to midnight UTC of the same day.
pub fn utc_start_of_day(secs: u64) -> u64 {
    (secs / SECS_PER_DAY) * SECS_PER_DAY
}

/// Midnight UTC of `date` in unix seconds: the day-aligned instant at which credit backed by a
/// bill maturing on `date` stops being credit.
pub fn credit_expires_at(date: BillDate) -> u64 {
    u64::try_from(date.midnight().assume_utc().unix_timestamp()).unwrap_or(0)
}

/// The credit/debit boundary shared by mint and validator: a keyset is matured (debit-like) when
/// it has no `final_expiry` or its expiry lies before the start of the current UTC day.
pub fn is_matured(final_expiry: Option<u64>, now: u64) -> bool {
    !matches!(final_expiry, Some(e) if e >= utc_start_of_day(now))
}

/// Returns the active keyset of the requested maturity (matured = debit-like), earliest expiry first.
pub fn active_keyset(
    infos: &[ecash::KeySetInfo],
    matured: bool,
    now: u64,
) -> Option<&ecash::KeySetInfo> {
    infos
        .iter()
        .filter(|info| info.active && is_matured(info.final_expiry, now) == matured)
        .min_by_key(|info| info.final_expiry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CURRENCY_UNIT;

    fn info(tag: u8, active: bool, final_expiry: Option<u64>) -> ecash::KeySetInfo {
        ecash::KeySetInfo {
            id: ecash::Id::V1([tag; 7]),
            unit: CURRENCY_UNIT,
            active,
            input_fee_ppk: 0,
            final_expiry,
        }
    }

    #[test]
    fn maturity_at_expiry_is_credit() {
        let day = 1_700_000_000;
        assert!(!is_matured(Some(day), day));
        assert!(is_matured(Some(day - SECS_PER_DAY), day));
        assert!(is_matured(None, day));
    }

    #[test]
    fn utc_start_of_day_floors_to_midnight() {
        let day = SECS_PER_DAY * 19_500;
        assert_eq!(utc_start_of_day(day), day);
        assert_eq!(utc_start_of_day(day + 1), day);
        assert_eq!(utc_start_of_day(day + SECS_PER_DAY - 1), day);
        assert_eq!(utc_start_of_day(day + SECS_PER_DAY), day + SECS_PER_DAY);
    }

    /// The predicate floors `now` itself, so callers that still pre-floor see no change.
    #[test]
    fn is_matured_is_invariant_within_a_day() {
        let day = SECS_PER_DAY * 19_500;
        for expiry in [
            Some(day - SECS_PER_DAY),
            Some(day),
            Some(day + SECS_PER_DAY),
            None,
        ] {
            for now in [day, day + 1, day + 11 * 3600, day + SECS_PER_DAY - 1] {
                assert_eq!(is_matured(expiry, now), is_matured(expiry, day));
            }
        }
    }

    /// Producer and consumer agree: credit for every second of the expiry day, debit from the next.
    #[test]
    fn credit_expires_at_flips_the_day_after_maturity() {
        let maturity = time::macros::date!(2026 - 08 - 03);
        let expiry = credit_expires_at(maturity);
        assert_eq!(expiry, 1_785_715_200);
        assert_eq!(utc_start_of_day(expiry), expiry);
        assert!(!is_matured(Some(expiry), expiry));
        assert!(!is_matured(Some(expiry), expiry + SECS_PER_DAY - 1));
        assert!(is_matured(Some(expiry), expiry + SECS_PER_DAY));
        assert_eq!(
            credit_expires_at(maturity.next_day().unwrap()),
            expiry + SECS_PER_DAY
        );
    }

    #[test]
    fn active_keyset_selects_by_class_and_earliest_expiry() {
        let now = SECS_PER_DAY * 19_500 + 3600;
        let infos = [
            info(1, true, None),
            info(2, true, Some(now - SECS_PER_DAY)),
            info(3, true, Some(now + 3 * SECS_PER_DAY)),
            info(4, true, Some(now + SECS_PER_DAY)),
            info(5, false, Some(now + SECS_PER_DAY / 2)),
        ];
        assert_eq!(active_keyset(&infos, true, now), Some(&infos[0]));
        assert_eq!(active_keyset(&infos, false, now), Some(&infos[3]));
        assert_eq!(active_keyset(&infos[2..], true, now), None);
        assert_eq!(active_keyset(&infos[..2], false, now), None);
    }
}
