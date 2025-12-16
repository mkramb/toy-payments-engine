use crate::engine::account::Account;
use crate::engine::transaction::{DepositRecord, Transaction};
use crate::{EngineError, Result};

use log::warn;
use rust_decimal::Decimal;

use super::TransactionProcessor;

impl TransactionProcessor {
    pub(super) fn process_deposit(&mut self, transaction: Transaction) -> Result<()> {
        let amount = transaction
            .amount
            .ok_or_else(|| EngineError::InvalidTransaction("Deposit missing amount".into()))?;

        if amount <= Decimal::ZERO {
            warn!(
                "Skipping deposit tx {} with non-positive amount: {}",
                transaction.tx, amount
            );
            return Ok(());
        }

        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        if account.locked {
            warn!(
                "Skipping deposit tx {} for locked account {}",
                transaction.tx, transaction.client
            );
            return Ok(());
        }

        account.available = account
            .available
            .checked_add(amount)
            .ok_or(EngineError::NumericOverflow)?;

        account.update_total();

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
            warn!(
                "Skipping withdrawal tx {} with non-positive amount: {}",
                transaction.tx, amount
            );
            return Ok(());
        }

        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        if account.locked {
            warn!(
                "Skipping withdrawal tx {} for locked account {}",
                transaction.tx, transaction.client
            );
            return Ok(());
        }

        if account.has_sufficient_funds(amount) {
            account.available = account
                .available
                .checked_sub(amount)
                .ok_or(EngineError::NumericUnderflow)?;

            account.update_total();
        }

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

    #[test]
    fn deposit_and_withdraw_updates_balances() {
        let mut processor = TransactionProcessor::new();

        processor.process(deposit(1, 1, "10.0")).unwrap();
        processor.process(withdrawal(1, 2, "4.5")).unwrap();

        let account = processor.accounts.get(&1).unwrap();

        assert_eq!(account.available, decimal("5.5"));
        assert_eq!(account.total, decimal("5.5"));
    }

    #[test]
    fn insufficient_funds_withdrawal_is_ignored() {
        let mut processor = TransactionProcessor::new();

        processor.process(deposit(1, 1, "5.0")).unwrap();
        processor.process(withdrawal(1, 2, "6.0")).unwrap();

        let account = processor.accounts.get(&1).unwrap();

        assert_eq!(account.available, decimal("5.0"));
        assert_eq!(account.total, decimal("5.0"));
    }

    #[test]
    fn missing_amount_returns_error() {
        let mut processor = TransactionProcessor::new();
        let deposit_missing_amount = Transaction {
            client: 1,
            tx_type: TransactionType::Deposit,
            tx: 1,
            amount: None,
        };

        let withdrawal_missing_amount = Transaction {
            client: 1,
            tx_type: TransactionType::Withdrawal,
            tx: 2,
            amount: None,
        };

        assert!(matches!(
            processor.process(deposit_missing_amount),
            Err(EngineError::InvalidTransaction(_))
        ));

        assert!(matches!(
            processor.process(withdrawal_missing_amount),
            Err(EngineError::InvalidTransaction(_))
        ));
    }
}
