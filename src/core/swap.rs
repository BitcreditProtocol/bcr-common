const FEE_RATE_PPK_MULTIPLIER: u64 = 1000;

#[cfg(any(feature = "wallet", test))]
pub mod wallet {
    // ----- standard library imports
    use std::collections::{HashMap, HashSet};
    // ----- extra library imports
    use cashu::{Amount, Id, KeySetInfo, Proof};
    use thiserror::Error;
    // ----- local imports
    use super::FEE_RATE_PPK_MULTIPLIER;

    // ----- end imports

    pub enum PaymentPlan<'a> {
        Ready {
            inputs: Vec<&'a Proof>,
            estimated_fee: Amount,
        },
        NeedSplit {
            proof: &'a Proof,
            target: cashu::amount::SplitTarget,
            estimated_fee: Amount,
        },
    }

    pub type SwapPlan = HashMap<cashu::Id, cashu::Amount>;

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
    /// WARNING: the function is basic and does not forward estimate fees for split proofs.
    /// - do not assume that  NeedSplit::estimated_fee is 100% accurate
    /// - do not assume that `NeedSplit` is potentially returned only once
    pub fn prepare_payment<'a>(
        proofs: &'a [Proof],
        target: Amount,
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<PaymentPlan<'a>> {
        // proofs must be sorted by amount in ascending order
        assert!(proofs.is_sorted_by_key(|p| p.amount));

        // TODO: calculated minimum fee more accurately
        const MINIMUM_FEE: cashu::Amount = cashu::Amount::ONE;
        const AVG_INPUT_SIZE: usize = 32; // based on cashu::Proof secret size, may vary
        // partition points, left partition: candidate for payment, right partition: candidate for split
        let gt_p = proofs.partition_point(|p| p.amount <= target + MINIMUM_FEE);
        let mut inputs: Vec<&Proof> = vec![];
        let mut inputs_total = Amount::ZERO;
        let mut inputs_size = 0;
        let mut payment_fee_rate_ppk = 0;
        // split candidate: we want to split the smallest proof gt remaining_target + fees
        let mut split: Option<&Proof> = None;
        for idx in (0..gt_p).rev() {
            let p = &proofs[idx];
            let kinfo = kinfos
                .get(&p.keyset_id)
                .ok_or(Error::UnknownKeyset(p.keyset_id))?;
            let new_fee_rate_ppk = std::cmp::max(payment_fee_rate_ppk, kinfo.input_fee_ppk);
            let new_inputs_size = inputs_size + p.secret.as_bytes().len();
            let new_fee = Amount::from(
                (new_fee_rate_ppk * new_inputs_size as u64).div_ceil(FEE_RATE_PPK_MULTIPLIER),
            );
            let new_total = inputs_total + p.amount;
            if new_total == target + new_fee {
                inputs.push(p);
                return Ok(PaymentPlan::Ready {
                    inputs,
                    estimated_fee: new_fee,
                });
            } else if new_total < target + new_fee {
                inputs_total = new_total;
                inputs_size = new_inputs_size;
                payment_fee_rate_ppk = new_fee_rate_ppk;
                inputs.push(p);
            } else {
                split = Some(p);
            }
        }
        let Some(split_proof) = split.or_else(|| proofs.get(gt_p)) else {
            let payment_fee = Amount::from(
                (payment_fee_rate_ppk * inputs_size as u64).div_ceil(FEE_RATE_PPK_MULTIPLIER),
            );
            return Err(Error::InsufficientBalance(
                inputs_total,
                target + payment_fee,
            ));
        };
        let split_fee_rate_ppk = kinfos
            .get(&split_proof.keyset_id)
            .ok_or(Error::UnknownKeyset(split_proof.keyset_id))?
            .input_fee_ppk;
        // the payment will contain proofs with fee_rate_ppk == split_fee_rate_ppk
        payment_fee_rate_ppk = std::cmp::max(payment_fee_rate_ppk, split_fee_rate_ppk);
        // payment size will contain at least one more simple proof
        let payment_size = inputs_size + AVG_INPUT_SIZE;
        let payment_fee = Amount::from(
            (payment_fee_rate_ppk * payment_size as u64).div_ceil(FEE_RATE_PPK_MULTIPLIER),
        );
        let split_fee = Amount::from(
            (split_fee_rate_ppk * split_proof.secret.as_bytes().len() as u64)
                .div_ceil(FEE_RATE_PPK_MULTIPLIER),
        );
        let split_target = cashu::amount::SplitTarget::Value(target + payment_fee - inputs_total);
        Ok(PaymentPlan::NeedSplit {
            proof: split_proof,
            target: split_target,
            estimated_fee: payment_fee + split_fee,
        })
    }

    pub fn verify_payment(
        inputs: &[Proof],
        target: Amount,
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<()> {
        let input_kids: HashSet<cashu::Id> = inputs.iter().map(|p| p.keyset_id).collect();
        let mut max_fee_rate_ppk = 0;
        for kid in input_kids {
            if !kinfos.contains_key(&kid) {
                return Err(Error::UnknownKeyset(kid));
            }
            let kinfo = kinfos.get(&kid).unwrap();
            max_fee_rate_ppk = std::cmp::max(max_fee_rate_ppk, kinfo.input_fee_ppk);
        }
        let total_secret_len: u64 = inputs
            .iter()
            .map(|p| p.secret.as_bytes().len() as u64)
            .sum();
        let required_fee =
            Amount::from((max_fee_rate_ppk * total_secret_len).div_ceil(FEE_RATE_PPK_MULTIPLIER));
        let total_input = inputs.iter().fold(Amount::ZERO, |acc, p| acc + p.amount);
        if total_input < target + required_fee {
            return Err(Error::InsufficientBalance(
                total_input,
                target + required_fee,
            ));
        }
        Ok(())
    }

    pub fn prepare_swap(inputs: &[Proof], kinfos: &HashMap<Id, KeySetInfo>) -> Result<SwapPlan> {
        let mut sum_by_id: HashMap<Id, Amount> = HashMap::new();
        let mut total_inputs_size = 0;
        for input in inputs {
            let entry = sum_by_id.entry(input.keyset_id).or_insert(Amount::ZERO);
            *entry += input.amount;
            total_inputs_size += input.secret.as_bytes().len() as u64;
        }
        let max_fee_rate_ppk = sum_by_id
            .keys()
            .map(|kid| kinfos.get(kid).unwrap().input_fee_ppk)
            .max()
            .unwrap_or(0);
        let mut required_fee =
            Amount::from((max_fee_rate_ppk * total_inputs_size).div_ceil(FEE_RATE_PPK_MULTIPLIER));
        let mut plan = SwapPlan::new();
        for (kid, mut amount) in sum_by_id {
            if amount <= required_fee {
                required_fee -= amount;
            } else {
                amount -= required_fee;
                required_fee = Amount::ZERO;
                plan.insert(kid, amount);
            }
        }
        Ok(plan)
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
    use super::FEE_RATE_PPK_MULTIPLIER;

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
        kinfos: &HashMap<Id, KeySetInfo>,
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
            let kinfo = kinfos
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
            if !kinfos.contains_key(keyset_id) {
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
        let required_fee =
            Amount::from((fee_rate_ppk * total_secret_len).div_ceil(FEE_RATE_PPK_MULTIPLIER));
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
        wallet::{PaymentPlan, SwapPlan, prepare_payment, prepare_swap, verify_payment},
    };
    use crate::core_tests;
    use cashu::Amount;
    use std::collections::HashMap;

    #[test]
    fn prepare_swap_inputs_1() {
        let target = Amount::from(5);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, PaymentPlan::Ready { .. }));
        let PaymentPlan::Ready {
            inputs,
            estimated_fee,
        } = result
        else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        verify_payment(&inputs, target, &kinfos).unwrap();
        assert_eq!(estimated_fee, Amount::ZERO);
    }

    #[test]
    fn prepare_swap_inputs_2() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, super::wallet::PaymentPlan::Ready { .. }));
        let PaymentPlan::Ready {
            inputs,
            estimated_fee,
        } = result
        else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        verify_payment(&inputs, target, &kinfos).unwrap();
        assert_eq!(estimated_fee, Amount::ZERO);
    }

    #[test]
    fn prepare_swap_inputs_3() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(2),
            Amount::from(4),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, super::wallet::PaymentPlan::Ready { .. }));
        let PaymentPlan::Ready {
            inputs,
            estimated_fee,
        } = result
        else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        verify_payment(&inputs, target, &kinfos).unwrap();
        assert_eq!(estimated_fee, Amount::ZERO);
    }

    #[test]
    fn prepare_swap_inputs_4() {
        let target = Amount::from(4);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, super::wallet::PaymentPlan::Ready { .. }));
        let PaymentPlan::Ready {
            inputs,
            estimated_fee,
        } = result
        else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        verify_payment(&inputs, target, &kinfos).unwrap();
        assert_eq!(estimated_fee, Amount::ZERO);
    }

    #[test]
    fn prepare_swap_fees_inputs_1() {
        let target = Amount::from(4);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(1), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, super::wallet::PaymentPlan::Ready { .. }));
        let PaymentPlan::Ready {
            inputs,
            estimated_fee,
        } = result
        else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        verify_payment(&inputs, target, &kinfos).unwrap();
        assert_eq!(estimated_fee, Amount::ONE);
    }

    #[test]
    fn prepare_swap_needsplit_1() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        assert!(matches!(result, PaymentPlan::NeedSplit { .. }));
        let PaymentPlan::NeedSplit {
            proof,
            target,
            estimated_fee,
        } = result
        else {
            panic!("expected NeedSplit");
        };
        assert_eq!(proof.amount, Amount::from(4));
        assert_eq!(target, cashu::amount::SplitTarget::Value(Amount::from(3)));
        assert_eq!(estimated_fee, Amount::ZERO);
    }

    #[test]
    fn prepare_swap_needsplit_2() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, cashu::Amount::from(7), &kinfos).unwrap();
        assert!(matches!(result, PaymentPlan::NeedSplit { .. }));
        let PaymentPlan::NeedSplit {
            proof,
            target,
            estimated_fee,
        } = result
        else {
            panic!("expected NeedSplit");
        };
        assert_eq!(proof.amount, Amount::from(8));
        assert_eq!(target, cashu::amount::SplitTarget::Value(Amount::from(1)));
        assert_eq!(estimated_fee, Amount::ZERO);
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
