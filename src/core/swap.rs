#[cfg(any(feature = "wallet", test))]
pub mod wallet {
    // ----- standard library imports
    use std::collections::HashMap;
    // ----- extra library imports
    use cashu::{Amount, Id, KeySetInfo, Proof};
    use thiserror::Error;
    // ----- local imports

    // ----- end imports

    pub enum SwapPrepared<'a> {
        Swap {
            inputs: Vec<&'a Proof>,
            outputs: HashMap<cashu::Id, cashu::Amount>,
        },
        NeedSplit {
            proof: &'a Proof,
            target: cashu::amount::SplitTarget,
        },
    }

    type Result<T> = std::result::Result<T, Error>;
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("keyset {0} not found in keysets")]
        UnknownKeyset(Id),
        #[error("total balance {0} is less than target {1}")]
        InsufficientBalance(Amount, Amount),
    }

    /// very basic coin selection for swap
    /// possible improvements:
    /// - knapsack algorithm for better selection
    /// - minimize fees
    /// - minimize overall expiration date, i.e. no expiration or expiring soon
    pub fn prepare_swap<'a>(
        proofs: &'a [Proof],
        target: Amount,
        keysets: &HashMap<Id, KeySetInfo>,
    ) -> Result<SwapPrepared<'a>> {
        const MINIMUM_FEE: cashu::Amount = cashu::Amount::ONE;
        assert!(proofs.is_sorted_by_key(|p| p.amount));
        let gt_p = proofs.partition_point(|p| p.amount <= target + MINIMUM_FEE);
        let mut inputs: Vec<&Proof> = vec![];
        let mut total = Amount::ZERO;
        let mut secret_size = 0;
        let mut fee_rate_ppk = 0;
        let mut split: Option<&Proof> = None;
        for idx in (0..gt_p).rev() {
            let p = &proofs[idx];
            let kinfo = keysets
                .get(&p.keyset_id)
                .ok_or(Error::UnknownKeyset(p.keyset_id))?;
            let new_fee_rate_ppk = std::cmp::max(fee_rate_ppk, kinfo.input_fee_ppk);
            let new_secret_size = secret_size + p.secret.as_bytes().len();
            let new_fee = Amount::from((new_fee_rate_ppk * new_secret_size as u64).div_ceil(1000));
            if total + p.amount == target + new_fee {
                inputs.push(p);
                let outputs = prepare_output(&inputs, new_fee);
                return Ok(SwapPrepared::Swap { inputs, outputs });
            } else if total + p.amount < target + new_fee {
                total += p.amount;
                secret_size = new_secret_size;
                fee_rate_ppk = new_fee_rate_ppk;
                inputs.push(p);
            } else {
                split = Some(p);
            }
        }
        let fees = Amount::from((fee_rate_ppk * secret_size as u64).div_ceil(1000));
        let split_target = cashu::amount::SplitTarget::Value(target + fees - total);
        if let Some(p) = split {
            return Ok(SwapPrepared::NeedSplit {
                proof: p,
                target: split_target,
            });
        }
        if gt_p >= proofs.len() {
            return Err(Error::InsufficientBalance(total, target));
        }
        Ok(SwapPrepared::NeedSplit {
            proof: &proofs[gt_p],
            target: split_target,
        })
    }

    fn prepare_output(inputs: &[&Proof], fee: cashu::Amount) -> HashMap<Id, Amount> {
        let mut outputs: HashMap<Id, Amount> = HashMap::new();
        for p in inputs {
            outputs
                .entry(p.keyset_id)
                .and_modify(|a| *a += p.amount)
                .or_insert(p.amount);
        }
        let mut remaining_fee = fee;
        for v in outputs.values_mut() {
            if remaining_fee == Amount::ZERO {
                break;
            }
            if *v > remaining_fee {
                *v -= remaining_fee;
                remaining_fee = Amount::ZERO;
            } else {
                remaining_fee -= *v;
                *v = Amount::ZERO;
            }
        }
        outputs
    }
}

#[cfg(any(feature = "mint", test))]
pub mod mint {
    // ----- standard library imports
    use std::collections::HashMap;
    // ----- extra library imports
    use cashu::{Amount, Id, KeySetInfo, Proof, ProofsMethods};
    use thiserror::Error;
    // ----- local imports

    // ----- end imports

    type Result<T> = std::result::Result<T, SwapVerificationError>;
    #[derive(Debug, Error)]
    pub enum SwapVerificationError {
        #[error("invalid input {0}")]
        InvalidInput(String),
        #[error("invalid output {0}")]
        InvalidOutput(String),
        #[error("keyset {0} not found in keysets")]
        UnknownKeyset(Id),
        #[error("InsufficientFees, required {0}, received {1}")]
        InsufficientFees(Amount, Amount),
        #[error("cashu::nut00: {0}")]
        Cdk00(#[from] cashu::nut00::Error),
    }

    pub fn verify_swap(
        inputs: &Vec<Proof>,
        outputs: &[cashu::BlindedMessage],
        keysets: &HashMap<Id, KeySetInfo>,
    ) -> Result<()> {
        // * no empty outputs
        if outputs.is_empty() {
            return Err(SwapVerificationError::InvalidOutput(String::from(
                "no outputs provided",
            )));
        }
        // * no empty inputs
        if inputs.is_empty() {
            return Err(SwapVerificationError::InvalidInput(String::from(
                "no inputs provided",
            )));
        }
        // * unique blinded_secrets
        for idx in 0..outputs.len() {
            let secret = outputs[idx].blinded_secret;
            let any_equal = outputs[idx + 1..]
                .iter()
                .any(|o| o.blinded_secret == secret);
            if any_equal {
                return Err(SwapVerificationError::InvalidOutput(String::from(
                    "duplicate blinded secrets",
                )));
            }
        }
        // per keyset verification
        let mut fee_rate_ppk = 0;
        let input_amounts = inputs.sum_by_keyset();
        let inputs_kids: Vec<cashu::Id> = input_amounts.keys().copied().collect();
        // * inputs keysets must be known
        for kid in inputs_kids {
            let kinfo = keysets
                .get(&kid)
                .ok_or(SwapVerificationError::UnknownKeyset(kid))?;
            fee_rate_ppk = std::cmp::max(fee_rate_ppk, kinfo.input_fee_ppk);
        }
        let mut output_amounts: HashMap<Id, Amount> = HashMap::new();
        for output in outputs {
            let entry = output_amounts
                .entry(output.keyset_id)
                .or_insert(Amount::ZERO);
            *entry += output.amount;
        }
        for (keyset_id, output_amount) in &output_amounts {
            // * no zero output
            if *output_amount == cashu::Amount::ZERO {
                return Err(SwapVerificationError::InvalidOutput(format!(
                    "output with keyset {keyset_id} has zero amount"
                )));
            }
            // * outputs keysets must be known
            if !keysets.contains_key(keyset_id) {
                return Err(SwapVerificationError::UnknownKeyset(*keyset_id));
            }
            // * corresponding input amount
            if !input_amounts.contains_key(keyset_id) {
                return Err(SwapVerificationError::InvalidInput(format!(
                    "no input for keyset {keyset_id}"
                )));
            }
            let input_amount = input_amounts
                .get(keyset_id)
                .copied()
                .unwrap_or(Amount::ZERO);
            // * input amount >= output amount
            if input_amount < *output_amount {
                return Err(SwapVerificationError::InvalidInput(format!(
                    "input amount {input_amount} for keyset {keyset_id} is less than output amount {output_amount}"
                )));
            }
        }
        let total_secret_len: u64 = inputs
            .iter()
            .map(|p| p.secret.as_bytes().len() as u64)
            .sum();
        let required_fee = Amount::from((fee_rate_ppk * total_secret_len).div_ceil(1000));
        let total_output = output_amounts
            .values()
            .fold(Amount::ZERO, |acc, x| acc + *x);
        let total_input = inputs.total_amount()?;
        if total_input < total_output + required_fee {
            return Err(SwapVerificationError::InsufficientFees(
                required_fee,
                total_input - total_output,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{
        mint::{SwapVerificationError, verify_swap},
        wallet::{SwapPrepared, prepare_swap},
    };
    use crate::core_tests;
    use cashu::Amount;
    use std::collections::HashMap;

    #[test]
    fn prepare_swap_inputs_1() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(5), &kinfos).unwrap();
        assert!(matches!(result, super::wallet::SwapPrepared::Swap { .. }));
    }

    #[test]
    fn prepare_swap_inputs_2() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(3), &kinfos).unwrap();
        assert!(matches!(result, super::wallet::SwapPrepared::Swap { .. }));
    }

    #[test]
    fn prepare_swap_inputs_3() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(2),
            Amount::from(4),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(3), &kinfos).unwrap();
        assert!(matches!(result, super::wallet::SwapPrepared::Swap { .. }));
    }

    #[test]
    fn prepare_swap_inputs_4() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(4), &kinfos).unwrap();
        assert!(matches!(result, super::wallet::SwapPrepared::Swap { .. }));
    }

    #[test]
    fn prepare_swap_fees_inputs_1() {
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(1), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(4), &kinfos).unwrap();
        assert!(matches!(result, super::wallet::SwapPrepared::Swap { .. }));
    }

    #[test]
    fn prepare_swap_needsplit_1() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(3), &kinfos).unwrap();
        assert!(matches!(result, SwapPrepared::NeedSplit { .. }));
        let SwapPrepared::NeedSplit { proof, target } = result else {
            panic!("expected NeedSplit");
        };
        assert_eq!(proof.amount, Amount::from(4));
        assert_eq!(target, cashu::amount::SplitTarget::Value(Amount::from(3)));
    }

    #[test]
    fn prepare_swap_needsplit_2() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_swap(&proofs, cashu::Amount::from(7), &kinfos).unwrap();
        assert!(matches!(result, SwapPrepared::NeedSplit { .. }));
        let SwapPrepared::NeedSplit { proof, target } = result else {
            panic!("expected NeedSplit");
        };
        assert_eq!(proof.amount, Amount::from(8));
        assert_eq!(target, cashu::amount::SplitTarget::Value(Amount::from(1)));
    }

    #[test]
    fn verify_swap_no_fee() {
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let input_amounts = vec![Amount::from(1), Amount::from(2)];
        let output_amounts = vec![Amount::from(2), Amount::from(1)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &input_amounts);
        let outputs: Vec<_> =
            core_tests::generate_random_ecash_blindedmessages(keyset.id, &output_amounts)
                .into_iter()
                .map(|(b, _, _)| b)
                .collect();
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = verify_swap(&proofs, &outputs, &kinfos);
        assert!(matches!(
            result,
            Err(SwapVerificationError::InsufficientFees(..))
        ));
    }
}
