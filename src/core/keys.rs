// ----- standard library imports
// ----- extra library imports
// ----- end imports

/// Zero-fee power-of-two denominations up to `max`.
pub fn fee_and_amounts(max: cashu::Amount) -> cashu::amount::FeeAndAmounts {
    let max = max.to_u64();
    let mut amounts = Vec::new();
    let mut d = 1u64;
    while d != 0 && d <= max {
        amounts.push(d);
        d <<= 1;
    }
    (0u64, amounts).into()
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
