// WARNING: we are using cashu::KeySetInfo struct where fee rate is indicated as parts per 1000,
// i.e. ppk, but we want to calculate fees with more precision, i.e. parts per 10000, ppk*10.
const FEE_RATE_PPK_MULTIPLIER: u64 = 10000;

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

        // partition points, left partition: candidate for payment, right partition: candidate for split
        let gt_p = proofs.partition_point(|p| p.amount <= target);
        let mut inputs: Vec<&Proof> = vec![];
        let mut inputs_total = Amount::ZERO;
        // split candidate: we want to split the smallest proof gt remaining_target + fees
        let mut split: Option<&Proof> = None;
        for idx in (0..gt_p).rev() {
            let p = &proofs[idx];
            let new_total = inputs_total + p.amount;
            if new_total == target {
                // we got the exact amount needed, stop here
                inputs.push(p);
                return Ok(PaymentPlan::Ready { inputs });
            } else if new_total < target {
                // not yet there, keep adding inputs
                inputs_total = new_total;
                inputs.push(p);
            } else if split.is_none() {
                // target exceeded, yet this proof is good for potential split
                split = Some(p);
            }
        }
        let split_target = target - inputs_total;
        let split_lt = can_proof_reach_target(split, target, kinfos)?;
        if let CanBeSplit::Yes { proof, split_fees } = split_lt {
            return Ok(PaymentPlan::NeedSplit {
                proof,
                target: cashu::amount::SplitTarget::Value(split_target),
                estimated_fee: split_fees,
            });
        }

        let split_gtp = can_proof_reach_target(proofs.get(gt_p), split_target, kinfos)?;
        if let CanBeSplit::Yes { proof, split_fees } = split_gtp {
            return Ok(PaymentPlan::NeedSplit {
                proof,
                target: cashu::amount::SplitTarget::Value(split_target),
                estimated_fee: split_fees,
            });
        }
        let split_max = can_proof_reach_target(proofs.last(), split_target, kinfos)?;
        match split_max {
            CanBeSplit::Yes { proof, split_fees } => Ok(PaymentPlan::NeedSplit {
                proof,
                target: cashu::amount::SplitTarget::Value(split_target),
                estimated_fee: split_fees,
            }),
            _ => Err(Error::InsufficientBalance(inputs_total, target)),
        }
    }

    enum CanBeSplit<'a> {
        No,
        Yes {
            proof: &'a Proof,
            split_fees: Amount,
        },
    }
    fn can_proof_reach_target<'a>(
        proof: Option<&'a Proof>,
        target: Amount,
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<CanBeSplit<'a>> {
        let Some(p) = proof else {
            return Ok(CanBeSplit::No);
        };
        let kinfo = kinfos
            .get(&p.keyset_id)
            .ok_or(Error::UnknownKeyset(p.keyset_id))?;
        let fee = Amount::from(
            (kinfo.input_fee_ppk * p.secret.as_bytes().len() as u64)
                .div_ceil(FEE_RATE_PPK_MULTIPLIER),
        );
        if p.amount >= target + fee {
            Ok(CanBeSplit::Yes {
                proof: p,
                split_fees: fee,
            })
        } else {
            Ok(CanBeSplit::No)
        }
    }

    pub fn required_fees(
        inputs: &[Proof],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<cashu::Amount> {
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
        Ok(required_fee)
    }

    pub fn prepare_swap(inputs: &[Proof], kinfos: &HashMap<Id, KeySetInfo>) -> Result<SwapPlan> {
        let mut sum_by_id: HashMap<Id, Amount> = HashMap::new();
        let mut total_inputs_size = 0;
        for input in inputs {
            if !kinfos.contains_key(&input.keyset_id) {
                return Err(Error::UnknownKeyset(input.keyset_id));
            }
            let entry = sum_by_id.entry(input.keyset_id).or_insert(Amount::ZERO);
            *entry += input.amount;
            total_inputs_size += input.secret.as_bytes().len() as u64;
        }
        let max_fee_rate_ppk = sum_by_id
            .keys()
            // unwrap is ok, we already checked this
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
    use std::collections::{HashMap, HashSet};
    // ----- extra library imports
    use cashu::{Amount, Id, KeySetInfo, Proof};
    use thiserror::Error;
    // ----- local imports
    use crate::core::{signature::ProofFingerprint, swap::FEE_RATE_PPK_MULTIPLIER};

    // ----- end imports

    type Result<T> = std::result::Result<T, VerificationError>;
    #[derive(Debug, Error)]
    pub enum VerificationError {
        #[error("invalid inputs {0}")]
        InvalidInputs(String),
        #[error("invalid outputs {0}")]
        InvalidOutputs(String),
        #[error("keyset {0} not found in keysets")]
        UnknownKeyset(Id),
        #[error("InsufficientFees, required {0}, received {1}")]
        InsufficientFees(Amount, Amount),
        #[error("cashu::nut00: {0}")]
        Cdk00(#[from] cashu::nut00::Error),
    }

    trait Input {
        fn y(&self) -> Result<cashu::PublicKey>;
        fn amount(&self) -> cashu::Amount;
        fn kid(&self) -> cashu::Id;
    }
    impl Input for cashu::Proof {
        fn y(&self) -> Result<cashu::PublicKey> {
            let y = self.y()?;
            Ok(y)
        }
        fn amount(&self) -> cashu::Amount {
            self.amount
        }
        fn kid(&self) -> cashu::Id {
            self.keyset_id
        }
    }
    impl Input for ProofFingerprint {
        fn y(&self) -> Result<cashu::PublicKey> {
            let y = cashu::PublicKey::from(self.y);
            Ok(y)
        }
        fn amount(&self) -> cashu::Amount {
            self.amount
        }
        fn kid(&self) -> cashu::Id {
            self.keyset_id
        }
    }

    fn verify_outputs(
        outputs: &[cashu::BlindedMessage],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<HashMap<cashu::Id, cashu::Amount>> {
        // * no empty outputs
        if outputs.is_empty() {
            return Err(VerificationError::InvalidOutputs(String::from(
                "no outputs provided",
            )));
        }
        // * unique blinded_secrets
        let b_secrets: HashSet<cashu::PublicKey> =
            outputs.iter().map(|output| output.blinded_secret).collect();
        if b_secrets.len() != outputs.len() {
            return Err(VerificationError::InvalidOutputs(String::from(
                "duplicate blinded secrets",
            )));
        }
        // amounts by keyset_id
        let mut amounts: HashMap<Id, Amount> = HashMap::new();
        for output in outputs {
            let entry = amounts.entry(output.keyset_id).or_insert(Amount::ZERO);
            *entry += output.amount;
        }
        for (kid, amount) in &amounts {
            // * no zero output
            if *amount == cashu::Amount::ZERO {
                return Err(VerificationError::InvalidOutputs(format!(
                    "zero output amount for {kid}"
                )));
            }
            // * outputs keysets must be known
            if !kinfos.contains_key(kid) {
                return Err(VerificationError::UnknownKeyset(*kid));
            }
        }
        Ok(amounts)
    }

    fn verify_inputs(
        inputs: &[impl Input],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<(u64, HashMap<Id, Amount>)> {
        // * no empty inputs
        if inputs.is_empty() {
            return Err(VerificationError::InvalidInputs(String::from(
                "no inputs provided",
            )));
        }
        // * unique fingerprints
        let ys: HashSet<cashu::PublicKey> = inputs
            .iter()
            .map(|input| input.y())
            .collect::<Result<_>>()?;
        if ys.len() != inputs.len() {
            return Err(VerificationError::InvalidInputs(String::from(
                "duplicate inputs",
            )));
        }
        // amounts by keyset_id
        let mut amounts: HashMap<Id, Amount> = HashMap::new();
        for input in inputs {
            let entry = amounts.entry(input.kid()).or_insert(Amount::ZERO);
            *entry += input.amount();
        }
        let mut fee_rate_ppk = 0;
        for (kid, amount) in &amounts {
            // * no zero input
            if *amount == cashu::Amount::ZERO {
                return Err(VerificationError::InvalidInputs(format!(
                    "zero input amount for {kid}"
                )));
            }
            // * inputs keysets must be known
            let kinfo = kinfos
                .get(kid)
                .ok_or(VerificationError::UnknownKeyset(*kid))?;
            fee_rate_ppk = std::cmp::max(fee_rate_ppk, kinfo.input_fee_ppk)
        }
        Ok((fee_rate_ppk, amounts))
    }

    pub fn verify_swap(
        inputs: &[Proof],
        outputs: &[cashu::BlindedMessage],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<()> {
        // verify outputs
        let output_amounts = verify_outputs(outputs, kinfos)?;
        // verify inputs
        let (fee_rate_ppk, input_amounts) = verify_inputs(inputs, kinfos)?;
        // per keyset verification
        for (kid, output_amount) in &output_amounts {
            // * corresponding input amount
            if !input_amounts.contains_key(kid) {
                return Err(VerificationError::InvalidInputs(format!(
                    "no input for keyset {kid}"
                )));
            }
            let input_amount = input_amounts.get(kid).copied().unwrap_or(Amount::ZERO);
            // * input amount >= output amount
            if input_amount < *output_amount {
                return Err(VerificationError::InvalidInputs(format!(
                    "{kid}: input {input_amount} < output {output_amount}"
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
        let total_input = input_amounts.values().fold(Amount::ZERO, |acc, x| acc + *x);
        if total_input < total_output + required_fee {
            return Err(VerificationError::InsufficientFees(
                required_fee,
                total_input - total_output,
            ));
        }
        Ok(())
    }

    pub fn verify_commit(
        inputs: &[ProofFingerprint],
        outputs: &[cashu::BlindedMessage],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<()> {
        // verify outputs
        let output_amounts = verify_outputs(outputs, kinfos)?;
        // verify inputs
        let (_, input_amounts) = verify_inputs(inputs, kinfos)?;
        // per keyset verification
        for (kid, output_amount) in &output_amounts {
            // * corresponding input amount
            if !input_amounts.contains_key(kid) {
                return Err(VerificationError::InvalidInputs(format!(
                    "no input for keyset {kid}"
                )));
            }
            let input_amount = input_amounts.get(kid).copied().unwrap_or(Amount::ZERO);
            // * input amount >= output amount
            if input_amount < *output_amount {
                return Err(VerificationError::InvalidInputs(format!(
                    "{kid}: input {input_amount} < output {output_amount}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{
        mint::{VerificationError, verify_swap},
        wallet::{PaymentPlan, prepare_payment, required_fees},
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
        let PaymentPlan::Ready { inputs } = result else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        let fees = required_fees(&inputs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ZERO);
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
        let PaymentPlan::Ready { inputs } = result else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        let fees = required_fees(&inputs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ZERO);
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
        let PaymentPlan::Ready { inputs } = result else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        let fees = required_fees(&inputs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ZERO);
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
        let PaymentPlan::Ready { inputs } = result else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        let fees = required_fees(&inputs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ZERO);
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
        let PaymentPlan::Ready { inputs } = result else {
            panic!("expected Ready");
        };
        let inputs: Vec<cashu::Proof> = inputs.iter().map(|p| *p).cloned().collect();
        let fees = required_fees(&inputs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ONE);
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
            Err(VerificationError::InsufficientFees(..))
        ));
    }

    #[test]
    fn verify_payment_9kbyte_1sat() {
        // 9kbyte inputs len is 1 sat in fees
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(1), Amount::from(2)];
        let mut proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        proofs[0].secret =
            cashu::secret::Secret::new(String::from_utf8(vec![0; 9 * 1024]).unwrap());
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let fees = required_fees(&proofs, &kinfos).unwrap();
        assert_eq!(fees, Amount::ONE);
    }

    #[test]
    fn verify_payment_11kbyte_2sat() {
        // 11kbyte inputs len is 2 sats in fees
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(4)];
        let mut proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        proofs[0].secret =
            cashu::secret::Secret::new(String::from_utf8(vec![0; 11 * 1024]).unwrap());
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let fees = required_fees(&proofs, &kinfos).unwrap();
        assert_eq!(fees, Amount::from(2));
    }
}
