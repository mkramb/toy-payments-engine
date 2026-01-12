use crate::engine::transaction::Transaction;
use crate::{EngineError, Result};

use super::TransactionProcessor;

impl TransactionProcessor {
    pub(super) fn process_dispute(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get_mut(&transaction.tx) {
            Some(d) => d,
            None => return Err(EngineError::TransactionNotFound(transaction.tx)),
        };

        if deposit.client_id != transaction.client {
            return Err(EngineError::FraudAttempt(
                transaction.client,
                transaction.tx,
                deposit.client_id,
            ));
        }

        if deposit.disputed {
            return Err(EngineError::DisputeAlreadyActive(transaction.tx));
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Err(EngineError::AccountNotFound(transaction.client)),
        };

        if account.locked {
            return Err(EngineError::AccountLocked(transaction.client));
        }

        let amount = deposit.amount;

        if !account.has_sufficient_funds(amount) {
            return Err(EngineError::InsufficientFunds(
                transaction.client,
                account.available.to_string(),
                amount.to_string(),
            ));
        }

        account.available = account
            .available
            .checked_sub(amount)
            .ok_or(EngineError::NumericUnderflow)?;

        account.held = account
            .held
            .checked_add(amount)
            .ok_or(EngineError::NumericOverflow)?;

        deposit.disputed = true;

        Ok(())
    }

    pub(super) fn process_resolve(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get_mut(&transaction.tx) {
            Some(d) => d,
            None => return Err(EngineError::TransactionNotFound(transaction.tx)),
        };

        if deposit.client_id != transaction.client {
            return Err(EngineError::FraudAttempt(
                transaction.client,
                transaction.tx,
                deposit.client_id,
            ));
        }

        if !deposit.disputed {
            return Err(EngineError::DisputeNotFound(transaction.tx));
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Err(EngineError::AccountNotFound(transaction.client)),
        };

        if account.locked {
            return Err(EngineError::AccountLocked(transaction.client));
        }

        let amount = deposit.amount;

        account.held = account
            .held
            .checked_sub(amount)
            .ok_or(EngineError::NumericUnderflow)?;

        account.available = account
            .available
            .checked_add(amount)
            .ok_or(EngineError::NumericOverflow)?;

        // Lets remove the resolved deposit to free memory, as it can't be disputed again
        self.deposits.remove(&transaction.tx);

        Ok(())
    }

    pub(super) fn process_chargeback(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get(&transaction.tx) {
            Some(d) => d,
            None => return Err(EngineError::TransactionNotFound(transaction.tx)),
        };

        if deposit.client_id != transaction.client {
            return Err(EngineError::FraudAttempt(
                transaction.client,
                transaction.tx,
                deposit.client_id,
            ));
        }

        if !deposit.disputed {
            return Err(EngineError::DisputeNotFound(transaction.tx));
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Err(EngineError::AccountNotFound(transaction.client)),
        };

        if account.locked {
            return Err(EngineError::AccountLocked(transaction.client));
        }

        let amount = deposit.amount;

        account.held = account
            .held
            .checked_sub(amount)
            .ok_or(EngineError::NumericUnderflow)?;

        account.locked = true;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::transaction::{Transaction, TransactionType};

    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn decimal(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    fn deposit(client: u16, tx: u32, amount: &str) -> Transaction {
        Transaction {
            client,
            tx_type: TransactionType::Deposit,
            tx,
            amount: Some(decimal(amount)),
        }
    }

    fn withdrawal(client: u16, tx: u32, amount: &str) -> Transaction {
        Transaction {
            client,
            tx_type: TransactionType::Withdrawal,
            tx,
            amount: Some(decimal(amount)),
        }
    }

    fn dispute(client: u16, tx: u32) -> Transaction {
        Transaction {
            client,
            tx_type: TransactionType::Dispute,
            tx,
            amount: None,
        }
    }

    fn resolve(client: u16, tx: u32) -> Transaction {
        Transaction {
            client,
            tx_type: TransactionType::Resolve,
            tx,
            amount: None,
        }
    }

    fn chargeback(client: u16, tx: u32) -> Transaction {
        Transaction {
            client,
            tx_type: TransactionType::Chargeback,
            tx,
            amount: None,
        }
    }

    mod dispute_tests {
        use super::*;

        #[test]
        fn moves_funds_from_available_to_held() {
            // Given: An account with a deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Disputing the deposit
            let result = processor.process(dispute(1, 100));

            // Then: Funds move from available to held
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, Decimal::ZERO);
            assert_eq!(account.held, decimal("50.0"));
            assert_eq!(account.total(), decimal("50.0"));

            // And: Deposit is marked as disputed
            let deposit_record = processor.deposits.get(&100).unwrap();
            assert!(deposit_record.disputed);
        }

        #[test]
        fn only_affects_disputed_deposit_amount() {
            // Given: Multiple deposits
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "30.0")).unwrap();
            processor.process(deposit(1, 101, "20.0")).unwrap();

            // When: Disputing only the first deposit
            let result = processor.process(dispute(1, 100));

            // Then: Only 30.0 is held, 20.0 remains available
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("20.0"));
            assert_eq!(account.held, decimal("30.0"));
            assert_eq!(account.total(), decimal("50.0"));
        }

        #[test]
        fn rejects_nonexistent_transaction() {
            // Given: An account with deposits
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Disputing a non-existent transaction
            let result = processor.process(dispute(1, 999));

            // Then: Returns TransactionNotFound error
            assert_eq!(result.unwrap_err(), EngineError::TransactionNotFound(999));
        }

        #[test]
        fn rejects_already_disputed_transaction() {
            // Given: A deposit already under dispute
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Disputing again
            let result = processor.process(dispute(1, 100));

            // Then: Returns DisputeAlreadyActive error
            assert_eq!(result.unwrap_err(), EngineError::DisputeAlreadyActive(100));
        }

        #[test]
        fn rejects_insufficient_available_funds() {
            // Given: A deposit that was partially withdrawn
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "100.0")).unwrap();
            processor.process(withdrawal(1, 101, "80.0")).unwrap();

            // When: Disputing original deposit (100.0) when only 20.0 available
            let result = processor.process(dispute(1, 100));

            // Then: Returns InsufficientFunds error with exact values
            match result {
                Err(EngineError::InsufficientFunds(client, available, required)) => {
                    assert_eq!(client, 1);
                    assert_eq!(available, "20.0");
                    assert_eq!(required, "100.0");
                }
                other => panic!("Expected InsufficientFunds error, got: {:?}", other),
            }

            // And: Deposit remains undisputed
            let deposit_record = processor.deposits.get(&100).unwrap();
            assert!(!deposit_record.disputed);
        }

        #[test]
        fn rejects_wrong_client_as_fraud() {
            // Given: Client 1 has a deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Client 2 tries to dispute client 1's transaction
            let result = processor.process(dispute(2, 100));

            // Then: Returns FraudAttempt error with exact details
            assert_eq!(
                result.unwrap_err(),
                EngineError::FraudAttempt(2, 100, 1) // attacker=2, tx=100, owner=1
            );

            // And: Deposit remains undisputed
            let deposit_record = processor.deposits.get(&100).unwrap();
            assert!(!deposit_record.disputed);

            // And: Client 1's balance is unchanged
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("50.0"));
            assert_eq!(account.held, Decimal::ZERO);
        }

        #[test]
        fn rejects_dispute_on_locked_account() {
            // Given: A locked account with remaining deposits
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(deposit(1, 101, "30.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();
            processor.process(chargeback(1, 100)).unwrap();

            // Verify account is locked
            assert!(processor.accounts.get(&1).unwrap().locked);

            // When: Trying to dispute another deposit on the locked account
            let result = processor.process(dispute(1, 101));

            // Then: Returns AccountLocked error
            assert_eq!(result.unwrap_err(), EngineError::AccountLocked(1));
        }
    }

    mod resolve_tests {
        use super::*;

        #[test]
        fn moves_funds_from_held_back_to_available() {
            // Given: A disputed deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Resolving the dispute
            let result = processor.process(resolve(1, 100));

            // Then: Funds return to available
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("50.0"));
            assert_eq!(account.held, Decimal::ZERO);
            assert_eq!(account.total(), decimal("50.0"));
        }

        #[test]
        fn removes_deposit_record_after_resolution() {
            // Given: A disputed deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Resolving the dispute
            processor.process(resolve(1, 100)).unwrap();

            // Then: Deposit record is removed (prevents re-dispute)
            assert!(processor.deposits.get(&100).is_none());
        }

        #[test]
        fn prevents_redispute_of_resolved_transaction() {
            // Given: A resolved dispute
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();
            processor.process(resolve(1, 100)).unwrap();

            // When: Trying to dispute again
            let result = processor.process(dispute(1, 100));

            // Then: Returns TransactionNotFound (deposit was removed)
            assert_eq!(result.unwrap_err(), EngineError::TransactionNotFound(100));
        }

        #[test]
        fn rejects_resolve_on_undisputed_transaction() {
            // Given: A deposit that was never disputed
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Trying to resolve
            let result = processor.process(resolve(1, 100));

            // Then: Returns DisputeNotFound error
            assert_eq!(result.unwrap_err(), EngineError::DisputeNotFound(100));
        }

        #[test]
        fn rejects_wrong_client_as_fraud() {
            // Given: Client 1 has a disputed deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Client 2 tries to resolve
            let result = processor.process(resolve(2, 100));

            // Then: Returns FraudAttempt error
            assert_eq!(result.unwrap_err(), EngineError::FraudAttempt(2, 100, 1));

            // And: Funds remain held
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, Decimal::ZERO);
            assert_eq!(account.held, decimal("50.0"));

            // And: Dispute remains active
            let deposit_record = processor.deposits.get(&100).unwrap();
            assert!(deposit_record.disputed);
        }
    }

    mod chargeback_tests {
        use super::*;

        #[test]
        fn deducts_from_held_and_locks_account() {
            // Given: A disputed deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Processing chargeback
            let result = processor.process(chargeback(1, 100));

            // Then: Held funds are removed, account is locked
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, Decimal::ZERO);
            assert_eq!(account.held, Decimal::ZERO);
            assert_eq!(account.total(), Decimal::ZERO);
            assert!(account.locked);
        }

        #[test]
        fn preserves_non_disputed_available_balance() {
            // Given: Multiple deposits, one disputed
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(deposit(1, 101, "30.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Processing chargeback on disputed deposit
            let result = processor.process(chargeback(1, 100));

            // Then: Non-disputed funds remain, account is locked
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("30.0"));
            assert_eq!(account.held, Decimal::ZERO);
            assert_eq!(account.total(), decimal("30.0"));
            assert!(account.locked);
        }

        #[test]
        fn blocks_all_future_transactions() {
            // Given: A chargebacked account
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();
            processor.process(chargeback(1, 100)).unwrap();

            // When/Then: All transaction types are rejected
            assert_eq!(
                processor.process(deposit(1, 101, "10.0")).unwrap_err(),
                EngineError::AccountLocked(1)
            );
            assert_eq!(
                processor.process(withdrawal(1, 102, "10.0")).unwrap_err(),
                EngineError::AccountLocked(1)
            );
        }

        #[test]
        fn rejects_chargeback_on_undisputed_transaction() {
            // Given: A deposit that was never disputed
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Trying to chargeback
            let result = processor.process(chargeback(1, 100));

            // Then: Returns DisputeNotFound error
            assert_eq!(result.unwrap_err(), EngineError::DisputeNotFound(100));

            // And: Account is not locked
            let account = processor.accounts.get(&1).unwrap();
            assert!(!account.locked);
        }

        #[test]
        fn rejects_wrong_client_as_fraud() {
            // Given: Client 1 has a disputed deposit
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor.process(dispute(1, 100)).unwrap();

            // When: Client 2 tries to chargeback
            let result = processor.process(chargeback(2, 100));

            // Then: Returns FraudAttempt error
            assert_eq!(result.unwrap_err(), EngineError::FraudAttempt(2, 100, 1));

            // And: Account is not locked
            let account = processor.accounts.get(&1).unwrap();
            assert!(!account.locked);

            // And: Funds remain held
            assert_eq!(account.held, decimal("50.0"));
        }
    }
}
