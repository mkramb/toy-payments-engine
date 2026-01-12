use crate::engine::account::Account;
use crate::engine::transaction::{DepositRecord, Transaction};
use crate::{EngineError, Result};

use rust_decimal::Decimal;

use super::TransactionProcessor;

impl TransactionProcessor {
    pub(super) fn process_deposit(&mut self, transaction: Transaction) -> Result<()> {
        let amount = transaction
            .amount
            .ok_or_else(|| EngineError::InvalidTransaction("Deposit missing amount".into()))?;

        if amount <= Decimal::ZERO {
            return Err(EngineError::InvalidTransaction(format!(
                "Deposit tx {} has non-positive amount: {}",
                transaction.tx, amount
            )));
        }

        if self.deposits.contains_key(&transaction.tx) {
            return Err(EngineError::DuplicateTransaction(transaction.tx));
        }

        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        if account.locked {
            return Err(EngineError::AccountLocked(transaction.client));
        }

        account.available = account
            .available
            .checked_add(amount)
            .ok_or(EngineError::NumericOverflow)?;

        self.deposits.insert(
            transaction.tx,
            DepositRecord::new(transaction.client, amount),
        );

        Ok(())
    }

    pub(super) fn process_withdrawal(&mut self, transaction: Transaction) -> Result<()> {
        let amount = transaction
            .amount
            .ok_or_else(|| EngineError::InvalidTransaction("Withdrawal missing amount".into()))?;

        if amount <= Decimal::ZERO {
            return Err(EngineError::InvalidTransaction(format!(
                "Withdrawal tx {} has non-positive amount: {}",
                transaction.tx, amount
            )));
        }

        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        if account.locked {
            return Err(EngineError::AccountLocked(transaction.client));
        }

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

    mod deposit_tests {
        use super::*;

        #[test]
        fn creates_account_for_new_client() {
            // Given: A new processor with no accounts
            let mut processor = TransactionProcessor::new();
            assert!(processor.accounts.is_empty());

            // When: Processing a deposit for client 1
            let result = processor.process(deposit(1, 100, "50.0"));

            // Then: Account is created with correct balance
            assert!(result.is_ok());
            assert_eq!(processor.accounts.len(), 1);

            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.client_id, 1);
            assert_eq!(account.available, decimal("50.0"));
            assert_eq!(account.held, Decimal::ZERO);
            assert_eq!(account.total(), decimal("50.0"));
            assert!(!account.locked);
        }

        #[test]
        fn adds_to_existing_account_balance() {
            // Given: An account with existing balance
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Processing another deposit
            let result = processor.process(deposit(1, 101, "25.0"));

            // Then: Balance is updated correctly
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("75.0"));
            assert_eq!(account.total(), decimal("75.0"));
        }

        #[test]
        fn stores_deposit_record_for_dispute_tracking() {
            // Given: A new processor
            let mut processor = TransactionProcessor::new();

            // When: Processing a deposit
            let result = processor.process(deposit(1, 100, "50.0"));

            // Then: Deposit record is stored with correct data
            assert!(result.is_ok());
            let deposit_record = processor.deposits.get(&100).unwrap();
            assert_eq!(deposit_record.client_id, 1);
            assert_eq!(deposit_record.amount, decimal("50.0"));
            assert!(!deposit_record.disputed);
        }

        #[test]
        fn rejects_duplicate_transaction_id() {
            // Given: A deposit with tx ID 100
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Attempting another deposit with same tx ID
            let result = processor.process(deposit(1, 100, "1000.0"));

            // Then: Returns DuplicateTransaction error with exact tx ID
            assert_eq!(result.unwrap_err(), EngineError::DuplicateTransaction(100));

            // And: Original balance is unchanged
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("50.0"));
        }

        #[test]
        fn rejects_deposit_to_locked_account() {
            // Given: A locked account (simulated via direct state setup)
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            processor
                .process(Transaction {
                    client: 1,
                    tx_type: TransactionType::Dispute,
                    tx: 100,
                    amount: None,
                })
                .unwrap();
            processor
                .process(Transaction {
                    client: 1,
                    tx_type: TransactionType::Chargeback,
                    tx: 100,
                    amount: None,
                })
                .unwrap();

            // Verify account is locked
            assert!(processor.accounts.get(&1).unwrap().locked);

            // When: Attempting to deposit to locked account
            let result = processor.process(deposit(1, 101, "25.0"));

            // Then: Returns AccountLocked error with exact client ID
            assert_eq!(result.unwrap_err(), EngineError::AccountLocked(1));
        }

        #[test]
        fn rejects_missing_amount() {
            // Given: A deposit transaction without amount
            let mut processor = TransactionProcessor::new();
            let tx = Transaction {
                client: 1,
                tx_type: TransactionType::Deposit,
                tx: 100,
                amount: None,
            };

            // When: Processing the transaction
            let result = processor.process(tx);

            // Then: Returns InvalidTransaction error
            match result {
                Err(EngineError::InvalidTransaction(msg)) => {
                    assert!(
                        msg.contains("missing amount"),
                        "Expected 'missing amount' in: {}",
                        msg
                    );
                }
                other => panic!("Expected InvalidTransaction error, got: {:?}", other),
            }
        }

        #[test]
        fn rejects_zero_amount() {
            // Given: A deposit with zero amount
            let mut processor = TransactionProcessor::new();
            let tx = Transaction {
                client: 1,
                tx_type: TransactionType::Deposit,
                tx: 100,
                amount: Some(Decimal::ZERO),
            };

            // When: Processing the transaction
            let result = processor.process(tx);

            // Then: Returns InvalidTransaction error
            match result {
                Err(EngineError::InvalidTransaction(msg)) => {
                    assert!(
                        msg.contains("non-positive"),
                        "Expected 'non-positive' in: {}",
                        msg
                    );
                }
                other => panic!("Expected InvalidTransaction error, got: {:?}", other),
            }
        }

        #[test]
        fn rejects_negative_amount() {
            // Given: A deposit with negative amount
            let mut processor = TransactionProcessor::new();
            let tx = Transaction {
                client: 1,
                tx_type: TransactionType::Deposit,
                tx: 100,
                amount: Some(decimal("-10.0")),
            };

            // When: Processing the transaction
            let result = processor.process(tx);

            // Then: Returns InvalidTransaction error
            match result {
                Err(EngineError::InvalidTransaction(msg)) => {
                    assert!(
                        msg.contains("non-positive"),
                        "Expected 'non-positive' in: {}",
                        msg
                    );
                }
                other => panic!("Expected InvalidTransaction error, got: {:?}", other),
            }
        }
    }

    mod withdrawal_tests {
        use super::*;

        #[test]
        fn deducts_from_available_balance() {
            // Given: An account with 100.0 balance
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "100.0")).unwrap();

            // When: Withdrawing 40.0
            let result = processor.process(withdrawal(1, 101, "40.0"));

            // Then: Balance is reduced correctly
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("60.0"));
            assert_eq!(account.held, Decimal::ZERO);
            assert_eq!(account.total(), decimal("60.0"));
        }

        #[test]
        fn allows_exact_balance_withdrawal() {
            // Given: An account with 50.0 balance
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Withdrawing exact balance
            let result = processor.process(withdrawal(1, 101, "50.0"));

            // Then: Balance is zero
            assert!(result.is_ok());
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, Decimal::ZERO);
            assert_eq!(account.total(), Decimal::ZERO);
        }

        #[test]
        fn rejects_insufficient_funds() {
            // Given: An account with 50.0 balance
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();

            // When: Attempting to withdraw 60.0
            let result = processor.process(withdrawal(1, 101, "60.0"));

            // Then: Returns InsufficientFunds error with exact values
            match result {
                Err(EngineError::InsufficientFunds(client, available, requested)) => {
                    assert_eq!(client, 1);
                    assert_eq!(available, "50.0");
                    assert_eq!(requested, "60.0");
                }
                other => panic!("Expected InsufficientFunds error, got: {:?}", other),
            }

            // And: Balance is unchanged
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("50.0"));
        }

        #[test]
        fn rejects_withdrawal_from_locked_account() {
            // Given: A locked account
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "100.0")).unwrap();
            processor
                .process(Transaction {
                    client: 1,
                    tx_type: TransactionType::Dispute,
                    tx: 100,
                    amount: None,
                })
                .unwrap();
            processor
                .process(Transaction {
                    client: 1,
                    tx_type: TransactionType::Chargeback,
                    tx: 100,
                    amount: None,
                })
                .unwrap();

            // When: Attempting to withdraw
            let result = processor.process(withdrawal(1, 101, "10.0"));

            // Then: Returns AccountLocked error
            assert_eq!(result.unwrap_err(), EngineError::AccountLocked(1));
        }

        #[test]
        fn rejects_missing_amount() {
            // Given: A withdrawal without amount
            let mut processor = TransactionProcessor::new();
            processor.process(deposit(1, 100, "50.0")).unwrap();
            let tx = Transaction {
                client: 1,
                tx_type: TransactionType::Withdrawal,
                tx: 101,
                amount: None,
            };

            // When: Processing the transaction
            let result = processor.process(tx);

            // Then: Returns InvalidTransaction error
            match result {
                Err(EngineError::InvalidTransaction(msg)) => {
                    assert!(
                        msg.contains("missing amount"),
                        "Expected 'missing amount' in: {}",
                        msg
                    );
                }
                other => panic!("Expected InvalidTransaction error, got: {:?}", other),
            }
        }

        #[test]
        fn creates_account_for_new_client_with_zero_balance() {
            // Given: A new processor
            let mut processor = TransactionProcessor::new();

            // When: Attempting withdrawal for non-existent client
            let result = processor.process(withdrawal(1, 100, "10.0"));

            // Then: Account is created but withdrawal fails due to insufficient funds
            match result {
                Err(EngineError::InsufficientFunds(client, available, requested)) => {
                    assert_eq!(client, 1);
                    assert_eq!(available, "0");
                    assert_eq!(requested, "10.0");
                }
                other => panic!("Expected InsufficientFunds error, got: {:?}", other),
            }

            // And: Account exists with zero balance
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, Decimal::ZERO);
        }
    }
}
