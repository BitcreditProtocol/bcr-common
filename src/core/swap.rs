// WARNING: we are using cashu::KeySetInfo struct where fee rate is indicated as parts per 1000,
// i.e. ppk, but we want to calculate fees with more precision, i.e. parts per 10000, ppk*10.
pub const FEE_RATE_PPK_MULTIPLIER: u64 = 10000;
// The maximum amount of inputs we add for one payment
const MAX_PAYMENT_INPUTS: usize = 32;

#[cfg(any(feature = "wallet", test))]
pub mod wallet {
    // ----- standard library imports
    use std::collections::{HashMap, HashSet};
    // ----- extra library imports
    use cashu::{Amount, Id, KeySetInfo, Proof};
    use thiserror::Error;
    // ----- local imports
    use super::{FEE_RATE_PPK_MULTIPLIER, MAX_PAYMENT_INPUTS};

    // ----- end imports

    #[derive(Debug)]
    pub enum PaymentPlan<'a> {
        Ready {
            inputs: Vec<&'a Proof>,
        },
        NeedSwap {
            inputs: Vec<&'a Proof>,
            target: cashu::amount::SplitTarget,
            estimated_fee: Amount,
        },
    }

    pub type SwapPlan = HashMap<cashu::Id, cashu::Amount>;

    pub type Result<T> = std::result::Result<T, Error>;
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("keyset {0} not found in keysets")]
        UnknownKeyset(Id),
        #[error("total balance {0} is less than target {1}")]
        InsufficientBalance(Amount, Amount),
    }

    /// Capped Smallest-first payment preparation.
    ///
    /// Strategy:
    /// * consume smaller proofs first
    /// * if an exact payment can be sent with inputs <= MAX_PAYMENT_INPUTS, return Ready with those inputs
    /// * otherwise return one NeedSwap with estimated fee, that creates payment outputs
    /// * sender-side execution requires at most one swap
    ///
    /// Trade Offs:
    /// * Optimizes for dust-collection over minimal amount of proofs per payment
    ///   This means, that for example, [2, 2, 4] with target = 4 will use [2, 2] instead of [4]
    /// * Creates more swaps than a best-fit selector, but we don't optimize for that primarily because of our DDoS-only fee
    /// * The MAX_PAYMENT_INPUTS parameter can be used to trade off amount of swaps vs. token size
    ///
    /// Important:
    /// `NeedSwap::estimated_fee` is an estimate based on the selected swap inputs
    /// `proofs` have to be sorted by amount ascending
    pub fn prepare_payment<'a>(
        proofs: &'a [Proof],
        target: Amount,
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<PaymentPlan<'a>> {
        // proofs must be sorted by amount in ascending order
        assert!(proofs.is_sorted_by_key(|p| p.amount));
        if target == Amount::ZERO {
            return Ok(PaymentPlan::Ready { inputs: vec![] });
        }
        let total_balance = proofs.iter().fold(Amount::ZERO, |acc, p| acc + p.amount);
        if total_balance < target {
            return Err(Error::InsufficientBalance(total_balance, target));
        }

        let mut inputs: Vec<&Proof> = vec![];
        let mut inputs_total = Amount::ZERO;

        let mut max_fee_rate_ppk = 0;
        let mut total_secret_len = 0;

        for proof in proofs {
            let kinfo = kinfos
                .get(&proof.keyset_id)
                .ok_or(Error::UnknownKeyset(proof.keyset_id))?;

            inputs.push(proof);
            inputs_total += proof.amount;

            max_fee_rate_ppk = std::cmp::max(max_fee_rate_ppk, kinfo.input_fee_ppk);
            total_secret_len += proof.secret.as_bytes().len() as u64;

            // if we hit the target exactly and are under our threshold for max inputs, we can just send the proofs
            if inputs_total == target && inputs.len() <= MAX_PAYMENT_INPUTS {
                return Ok(PaymentPlan::Ready { inputs });
            }

            let estimated_fee = Amount::from(
                (max_fee_rate_ppk * total_secret_len).div_ceil(FEE_RATE_PPK_MULTIPLIER),
            );
            // otherwise, if we're over the target + fee, we have to do a swap
            // this swap will get us the payment proofs, pay the fee and optionally return some change
            if inputs_total >= target + estimated_fee {
                return Ok(PaymentPlan::NeedSwap {
                    inputs,
                    target: cashu::amount::SplitTarget::Value(target),
                    estimated_fee,
                });
            }
        }

        // enough for target, but not for target + fee
        let estimated_fee =
            Amount::from((max_fee_rate_ppk * total_secret_len).div_ceil(FEE_RATE_PPK_MULTIPLIER));
        Err(Error::InsufficientBalance(
            total_balance,
            target + estimated_fee,
        ))
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

    #[cfg(any(test, all(feature = "wallet", feature = "mint")))]
    pub fn prepare_signed_swap(
        inputs: &[Proof],
        kinfos: &HashMap<Id, KeySetInfo>,
    ) -> Result<SwapPlan> {
        _prepare_swap(inputs, kinfos, true)
    }

    pub fn prepare_swap(inputs: &[Proof], kinfos: &HashMap<Id, KeySetInfo>) -> Result<SwapPlan> {
        _prepare_swap(inputs, kinfos, false)
    }

    fn _prepare_swap(
        inputs: &[Proof],
        kinfos: &HashMap<Id, KeySetInfo>,
        no_fees: bool,
    ) -> Result<SwapPlan> {
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
        if no_fees {
            required_fee = Amount::ZERO;
        }
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

    /// we prioritize expired credits first to reduce the circulating supply of such keysets
    /// second we select among debit keysets
    /// last we fill the missing amount with credits proofs
    /// if the target amount cannot be reached, the function returns the closest amount possible
    /// either slightly below or above the target, depending on the available proofs
    /// No swap plan is returned
    pub fn prepare_melt<'a>(
        proofs: &'a [Proof],
        kinfos: &HashMap<Id, KeySetInfo>,
        target: Amount,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<&'a cashu::Proof>> {
        if target == Amount::ZERO {
            return Ok(vec![]);
        }
        if proofs.is_empty() {
            return Err(Error::InsufficientBalance(Amount::ZERO, target));
        }
        assert!(proofs.is_sorted_by_key(|p| p.amount));
        let mut pure_debits: Vec<&Proof> = Vec::with_capacity(proofs.len());
        let mut expire_credits: Vec<&Proof> = Vec::with_capacity(proofs.len());
        let mut credits: Vec<&Proof> = Vec::with_capacity(proofs.len());
        let now = now.timestamp() as u64;
        for p in proofs {
            let kinfo = kinfos
                .get(&p.keyset_id)
                .ok_or(Error::UnknownKeyset(p.keyset_id))?;
            match kinfo.final_expiry {
                None => pure_debits.push(p),
                Some(expiry) if expiry < now => expire_credits.push(p),
                Some(_) => credits.push(p),
            }
        }
        let mut selection: Vec<&Proof> = Vec::with_capacity(proofs.len());
        let mut closest: Option<&Proof> = None;
        let selection_amount = select_coin_inner(
            expire_credits.as_slice(),
            &mut selection,
            target,
            &mut closest,
        );
        if selection_amount == target {
            return Ok(selection);
        }
        let selection_amount =
            select_coin_inner(pure_debits.as_slice(), &mut selection, target, &mut closest);
        if selection_amount == target {
            return Ok(selection);
        }
        let selection_amount =
            select_coin_inner(credits.as_slice(), &mut selection, target, &mut closest);
        if selection_amount == target {
            return Ok(selection);
        }
        // which one is closer?
        let Some(closest) = closest else {
            return Ok(selection);
        };
        // we prefer overshooting to target
        if (target - selection_amount) >= (selection_amount + closest.amount - target) {
            selection.push(closest);
        }
        Ok(selection)
    }

    fn select_coin_inner<'a>(
        candidates: &[&'a Proof],
        selection: &mut Vec<&'a Proof>,
        target: Amount,
        closest: &mut Option<&'a Proof>,
    ) -> Amount {
        let mut selection_amount = selection.iter().fold(Amount::ZERO, |acc, p| acc + p.amount);
        for p in candidates.iter().rev() {
            let new_amount = selection_amount + p.amount;
            if new_amount == target {
                selection.push(p);
                return new_amount;
            } else if new_amount < target {
                selection_amount = new_amount;
                selection.push(p);
            } else if let Some(closest_p) = closest {
                if new_amount < selection_amount + closest_p.amount {
                    closest.replace(p);
                }
            } else {
                closest.replace(p);
            }
        }
        selection_amount
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
        #[error("InsufficientInputs, inputs {0}, fees {1}, outputs {2}")]
        InsufficientInputs(Amount, Amount, Amount),
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

    pub enum FeePolicy {
        Apply,
        Ignore,
    }

    pub fn verify_swap(
        inputs: &[Proof],
        outputs: &[cashu::BlindedMessage],
        kinfos: &HashMap<Id, KeySetInfo>,
        fee_policy: FeePolicy,
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
        let fee_rate_ppk = match fee_policy {
            FeePolicy::Apply => fee_rate_ppk,
            FeePolicy::Ignore => 0,
        };
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
            return Err(VerificationError::InsufficientInputs(
                total_input,
                required_fee,
                total_output,
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
        mint::{FeePolicy, VerificationError, verify_swap},
        wallet::{Error as WalletError, PaymentPlan, prepare_melt, prepare_payment, required_fees},
    };
    use crate::core_tests;
    use cashu::Amount;
    use std::collections::HashMap;

    // HELPERS
    fn split_target_value(target: cashu::amount::SplitTarget) -> Amount {
        let cashu::amount::SplitTarget::Value(amount) = target else {
            panic!("expected value split target");
        };
        amount
    }

    fn selected_amounts(inputs: &[&cashu::Proof]) -> Vec<Amount> {
        inputs.iter().map(|p| p.amount).collect()
    }

    fn sum_inputs(inputs: &[&cashu::Proof]) -> Amount {
        inputs
            .iter()
            .fold(Amount::ZERO, |acc, proof| acc + proof.amount)
    }

    fn assert_ready(plan: PaymentPlan<'_>, target: Amount) -> Vec<Amount> {
        let PaymentPlan::Ready { inputs } = plan else {
            panic!("expected Ready");
        };
        assert!(inputs.len() <= super::MAX_PAYMENT_INPUTS);
        assert_eq!(sum_inputs(&inputs), target);
        selected_amounts(&inputs)
    }

    fn assert_needswap(
        plan: PaymentPlan<'_>,
        payment_target: Amount,
    ) -> (Vec<Amount>, Amount, Amount) {
        let PaymentPlan::NeedSwap {
            inputs,
            target,
            estimated_fee,
        } = plan
        else {
            panic!("expected NeedSwap");
        };
        let input_total = sum_inputs(&inputs);
        let payment = split_target_value(target);
        assert_eq!(payment, payment_target);
        assert!(input_total >= payment + estimated_fee);
        (selected_amounts(&inputs), payment, estimated_fee)
    }

    // PREPARE PAYMENT
    #[test]
    fn prepare_payment_checks_keyset() {
        let target = Amount::ONE;
        let (_kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::new();
        let result = prepare_payment(&proofs, target, &kinfos);
        assert!(matches!(
                result,
                Err(WalletError::UnknownKeyset(kid)) if kid == keyset.id
        ));
    }

    #[test]
    fn prepare_payment_empty_proofs_nonzero_target() {
        let target = Amount::ONE;
        let kinfos = HashMap::new();
        let proofs = vec![];
        let result = prepare_payment(&proofs, target, &kinfos);
        assert!(matches!(
            result,
            Err(WalletError::InsufficientBalance(balance, required))
                if balance == Amount::ZERO && required == target
        ));
    }

    #[test]
    fn prepare_payment_inputs_empty() {
        let target = Amount::from(0);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert!(selected.is_empty());
    }

    #[test]
    fn prepare_payment_target_greater_balance() {
        let target = Amount::from(10);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos);
        assert!(matches!(result, Err(WalletError::InsufficientBalance(..))));
    }

    #[test]
    fn prepare_payment_inputs_1() {
        let target = Amount::from(5);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::from(1), Amount::from(4)]);
    }

    #[test]
    fn prepare_payment_inputs_2() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::from(1), Amount::from(2)]);
    }

    #[test]
    fn prepare_payment_inputs_3() {
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
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::from(1), Amount::from(2)]);
    }

    #[test]
    fn prepare_payment_inputs_4() {
        let target = Amount::from(4);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::from(4)]);
    }

    #[test]
    fn prepare_payment_fees_inputs_1() {
        let target = Amount::from(4);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(1), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::from(1), Amount::from(4)]);
        assert_eq!(payment, Amount::from(4));
        assert_eq!(fee, Amount::ONE);
    }

    #[test]
    fn prepare_payment_needswap_1() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::from(4)]);
        assert_eq!(payment, Amount::from(3));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_needswap_2() {
        let target = Amount::from(7);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)]; // 14
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![Amount::from(2), Amount::from(4), Amount::from(8)]
        );
        assert_eq!(payment, Amount::from(7));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_needswap_2_fee() {
        let target = Amount::from(7);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)]; // 14
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![Amount::from(2), Amount::from(4), Amount::from(8)]
        );
        assert_eq!(payment, Amount::from(7));
        assert_eq!(fee, Amount::ONE);
    }

    #[test]
    fn prepare_payment_very_little_funds_exact() {
        let target = Amount::ONE;
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::ONE]);
    }

    #[test]
    fn prepare_payment_very_little_funds_insufficient() {
        let target = Amount::from(2);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos);
        assert!(matches!(result, Err(WalletError::InsufficientBalance(..))));
    }

    #[test]
    fn prepare_payment_single_big_proof_returns_needswap_with_change() {
        let target = Amount::from(30);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(128)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::from(128)]);
        assert_eq!(payment, Amount::from(30));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_exact_small_proof_payment_consumes_smallest_first() {
        let target = Amount::from(4);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(1),
            Amount::from(2),
            Amount::from(4),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(
            selected,
            vec![Amount::from(1), Amount::from(1), Amount::from(2)]
        );
    }

    #[test]
    fn prepare_payment_small_proofs_overshoot_returns_needswap() {
        let target = Amount::from(6);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![Amount::from(1), Amount::from(2), Amount::from(4)]
        );
        assert_eq!(payment, Amount::from(6));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_only_small_proofs_under_direct_input_cap() {
        let target = Amount::from(super::MAX_PAYMENT_INPUTS as u64);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE; super::MAX_PAYMENT_INPUTS];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::ONE; super::MAX_PAYMENT_INPUTS]);
    }

    #[test]
    fn prepare_payment_only_small_proofs_over_direct_input_cap_returns_needswap() {
        let input_count = super::MAX_PAYMENT_INPUTS + 1;
        let target = Amount::from(input_count as u64);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE; input_count];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::ONE; input_count]);
        assert_eq!(payment, target);
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_only_small_proofs_large_swap() {
        let input_count = 1000;
        let target = Amount::from(900);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE; input_count];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected.len(), 900);
        assert!(selected.iter().all(|amount| *amount == Amount::ONE));
        assert_eq!(payment, target);
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_well_distributed_exact() {
        let target = Amount::from(15);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(4),
            Amount::from(8),
            Amount::from(16),
            Amount::from(32),
            Amount::from(64),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(
            selected,
            vec![
                Amount::from(1),
                Amount::from(2),
                Amount::from(4),
                Amount::from(8),
            ]
        );
    }

    #[test]
    fn prepare_payment_well_distributed_overshoot_returns_needswap() {
        let target = Amount::from(20);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(4),
            Amount::from(8),
            Amount::from(16),
            Amount::from(32),
            Amount::from(64),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![
                Amount::from(1),
                Amount::from(2),
                Amount::from(4),
                Amount::from(8),
                Amount::from(16),
            ]
        );
        assert_eq!(payment, Amount::from(20));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_lots_of_funds_uses_smallest_until_funded() {
        let target = Amount::from(512);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(4),
            Amount::from(8),
            Amount::from(16),
            Amount::from(32),
            Amount::from(64),
            Amount::from(128),
            Amount::from(256),
            Amount::from(512),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![
                Amount::from(1),
                Amount::from(2),
                Amount::from(4),
                Amount::from(8),
                Amount::from(16),
                Amount::from(32),
                Amount::from(64),
                Amount::from(128),
                Amount::from(256),
                Amount::from(512),
            ]
        );
        assert_eq!(payment, Amount::from(512));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_lots_of_funds_uses_small_first() {
        let target = Amount::from(100);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            Amount::from(1),
            Amount::from(2),
            Amount::from(4),
            Amount::from(8),
            Amount::from(16),
            Amount::from(32),
            Amount::from(64),
            Amount::from(128),
            Amount::from(256),
            Amount::from(512),
        ];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![
                Amount::from(1),
                Amount::from(2),
                Amount::from(4),
                Amount::from(8),
                Amount::from(16),
                Amount::from(32),
                Amount::from(64),
            ]
        );
        assert_eq!(payment, Amount::from(100));
        assert_eq!(fee, Amount::ZERO);
    }

    #[test]
    fn prepare_payment_fee_exact_payment_returns_ready() {
        let target = Amount::from(128);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(128)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::from(128)]);
    }

    #[test]
    fn prepare_payment_fee_needswap_fee_consumes_remaining_amount() {
        let target = Amount::from(127);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(128)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::from(128)]);
        assert_eq!(payment, Amount::from(127));
        assert_eq!(fee, Amount::ONE);
    }

    #[test]
    fn prepare_payment_duplicate_amounts_exact() {
        let target = Amount::from(3);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::ONE, Amount::ONE, Amount::ONE];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let selected = assert_ready(result, target);
        assert_eq!(selected, vec![Amount::ONE, Amount::ONE, Amount::ONE]);
    }

    #[test]
    fn prepare_payment_enough_for_target_but_not_for_swap_fee() {
        let target = Amount::from((super::MAX_PAYMENT_INPUTS + 1) as u64);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::ONE; super::MAX_PAYMENT_INPUTS + 1];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos);
        assert!(matches!(
            result,
            Err(WalletError::InsufficientBalance(balance, required))
                if balance == target && required == target + Amount::ONE
        ));
    }

    #[test]
    fn prepare_payment_needswap_with_two_sat_fee() {
        let target = Amount::from(126);
        let (mut kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        kinfo.input_fee_ppk = 1;
        let amounts = vec![Amount::from(128)];
        let mut proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        proofs[0].secret = cashu::secret::Secret::new("x".repeat(11 * 1024));
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(selected, vec![Amount::from(128)]);
        assert_eq!(payment, Amount::from(126));
        assert_eq!(fee, Amount::from(2));
    }

    #[test]
    fn prepare_payment_prefers_smallest_prefix_over_later_exact_match() {
        let target = Amount::from(8);
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_payment(&proofs, target, &kinfos).unwrap();
        let (selected, payment, fee) = assert_needswap(result, target);
        assert_eq!(
            selected,
            vec![Amount::from(2), Amount::from(4), Amount::from(8)]
        );
        assert_eq!(payment, Amount::from(8));
        assert_eq!(fee, Amount::ZERO);
    }

    // VERIFY SWAP
    #[test]
    fn prepare_melt_all_expired() {
        let (mut kinfo_expired, keyset_expired) = core_tests::generate_random_ecash_keyset();
        let (kinfo_debit, keyset_debit) = core_tests::generate_random_ecash_keyset();
        let (mut kinfo_credit, keyset_credit) = core_tests::generate_random_ecash_keyset();
        let now = chrono::Utc::now();
        kinfo_expired.final_expiry =
            Some((now - chrono::Duration::seconds(3600)).timestamp() as u64);
        kinfo_credit.final_expiry =
            Some((now + chrono::Duration::seconds(3600)).timestamp() as u64);
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs_expired = core_tests::generate_random_ecash_proofs(&keyset_expired, &amounts);
        let proofs_debit = core_tests::generate_random_ecash_proofs(&keyset_debit, &amounts);
        let proofs_credit = core_tests::generate_random_ecash_proofs(&keyset_credit, &amounts);
        let kinfos = HashMap::from([
            (keyset_expired.id, cashu::KeySetInfo::from(kinfo_expired)),
            (keyset_debit.id, cashu::KeySetInfo::from(kinfo_debit)),
            (keyset_credit.id, cashu::KeySetInfo::from(kinfo_credit)),
        ]);
        let mut proofs =
            Vec::with_capacity(proofs_expired.len() + proofs_debit.len() + proofs_credit.len());
        proofs.extend(proofs_expired);
        proofs.extend(proofs_debit);
        proofs.extend(proofs_credit);
        proofs.sort_by_key(|p| p.amount);
        let result = prepare_melt(&proofs, &kinfos, Amount::from(5), now).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].keyset_id, keyset_expired.id);
        assert_eq!(result[1].keyset_id, keyset_expired.id);
    }

    #[test]
    fn prepare_melt_expired_debit() {
        let (mut kinfo_expired, keyset_expired) = core_tests::generate_random_ecash_keyset();
        let (kinfo_debit, keyset_debit) = core_tests::generate_random_ecash_keyset();
        let (mut kinfo_credit, keyset_credit) = core_tests::generate_random_ecash_keyset();
        let now = chrono::Utc::now();
        kinfo_expired.final_expiry =
            Some((now - chrono::Duration::seconds(3600)).timestamp() as u64);
        kinfo_credit.final_expiry =
            Some((now + chrono::Duration::seconds(3600)).timestamp() as u64);
        let amounts = vec![Amount::from(2), Amount::from(4), Amount::from(8)];
        let proofs_expired = core_tests::generate_random_ecash_proofs(&keyset_expired, &amounts);
        let proofs_credit = core_tests::generate_random_ecash_proofs(&keyset_credit, &amounts);
        let amounts = [Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs_debit = core_tests::generate_random_ecash_proofs(&keyset_debit, &amounts);
        let kinfos = HashMap::from([
            (keyset_expired.id, cashu::KeySetInfo::from(kinfo_expired)),
            (keyset_debit.id, cashu::KeySetInfo::from(kinfo_debit)),
            (keyset_credit.id, cashu::KeySetInfo::from(kinfo_credit)),
        ]);
        let mut proofs =
            Vec::with_capacity(proofs_expired.len() + proofs_debit.len() + proofs_credit.len());
        proofs.extend(proofs_expired);
        proofs.extend(proofs_debit);
        proofs.extend(proofs_credit);
        proofs.sort_by_key(|p| p.amount);
        let result = prepare_melt(&proofs, &kinfos, Amount::from(5), now).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].keyset_id, keyset_expired.id);
        assert_eq!(result[1].keyset_id, keyset_debit.id);
    }

    #[test]
    fn prepare_melt_not_enough_balance() {
        let (mut kinfo_expired, keyset_expired) = core_tests::generate_random_ecash_keyset();
        let (kinfo_debit, keyset_debit) = core_tests::generate_random_ecash_keyset();
        let (mut kinfo_credit, keyset_credit) = core_tests::generate_random_ecash_keyset();
        let now = chrono::Utc::now();
        kinfo_expired.final_expiry =
            Some((now - chrono::Duration::seconds(3600)).timestamp() as u64);
        kinfo_credit.final_expiry =
            Some((now + chrono::Duration::seconds(3600)).timestamp() as u64);
        let amounts = vec![Amount::from(1), Amount::from(2), Amount::from(4)];
        let proofs_expired = core_tests::generate_random_ecash_proofs(&keyset_expired, &amounts);
        let proofs_credit = core_tests::generate_random_ecash_proofs(&keyset_credit, &amounts);
        let proofs_debit = core_tests::generate_random_ecash_proofs(&keyset_debit, &amounts);
        let kinfos = HashMap::from([
            (keyset_expired.id, cashu::KeySetInfo::from(kinfo_expired)),
            (keyset_debit.id, cashu::KeySetInfo::from(kinfo_debit)),
            (keyset_credit.id, cashu::KeySetInfo::from(kinfo_credit)),
        ]);
        let mut proofs =
            Vec::with_capacity(proofs_expired.len() + proofs_debit.len() + proofs_credit.len());
        proofs.extend(proofs_expired);
        proofs.extend(proofs_debit);
        proofs.extend(proofs_credit);
        proofs.sort_by_key(|p| p.amount);
        let result = prepare_melt(&proofs, &kinfos, Amount::from(30), now).unwrap();
        assert_eq!(result.len(), 9);
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
        let result = verify_swap(&proofs, &outputs, &kinfos, FeePolicy::Apply);
        assert!(matches!(
            result,
            Err(VerificationError::InsufficientInputs(..))
        ));
    }

    #[test]
    fn verify_swap_no_fee_ignore_policy() {
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
        verify_swap(&proofs, &outputs, &kinfos, FeePolicy::Ignore).unwrap();
    }

    // REQUIRED FEES
    #[test]
    fn required_fees_9kbyte_1sat() {
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
    fn required_fees_11kbyte_2sat() {
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

    #[test]
    fn prepare_melt_close_to_target() {
        let (kinfo, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![Amount::from(2), Amount::from(2), Amount::from(4)];
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amounts);
        let kinfos = HashMap::from([(keyset.id, cashu::KeySetInfo::from(kinfo))]);
        let result = prepare_melt(&proofs, &kinfos, Amount::from(7), chrono::Utc::now()).unwrap();
        assert_eq!(result.len(), 3);
    }
}
