// ----- standard library imports
// ----- extra library imports
// ----- end imports

/// Denominations and input fee of a keyset, for splitting amounts.
pub fn to_fee_and_amounts(keyset: &cashu::KeySet) -> cashu::amount::FeeAndAmounts {
    let amounts = keyset
        .keys
        .iter()
        .map(|(amount, _)| amount.to_u64())
        .collect();
    (keyset.input_fee_ppk, amounts).into()
}

/// Build a public KeySet from a MintKeySet.
pub fn to_keyset(keyset: &cashu::MintKeySet, active: Option<bool>) -> cashu::KeySet {
    cashu::KeySet {
        id: keyset.id,
        unit: keyset.unit.clone(),
        active,
        keys: keyset.keys.clone().into(),
        input_fee_ppk: keyset.input_fee_ppk,
        final_expiry: keyset.final_expiry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_utils::generate_random_ecash_keyset;

    #[test]
    fn test_to_fee_and_amounts() {
        let (info, set) = generate_random_ecash_keyset();
        let mut keyset = to_keyset(&set, None);
        keyset.input_fee_ppk = 100;

        let fee_and_amounts = to_fee_and_amounts(&keyset);
        assert_eq!(fee_and_amounts.amounts(), info.amounts.as_slice());
        assert_eq!(fee_and_amounts.fee(), keyset.input_fee_ppk);
    }
}
