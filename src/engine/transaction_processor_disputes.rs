use crate::engine::transaction::Transaction;
use crate::{EngineError, Result};

use super::TransactionProcessor;

impl TransactionProcessor {
    pub(super) fn process_dispute(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get_mut(&transaction.tx) {
            Some(d) => d,
            None => return Ok(()),
        };

        if deposit.client_id != transaction.client || deposit.disputed {
            return Ok(());
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Ok(()),
        };

        if account.locked {
            return Ok(());
        }

        let amount = deposit.amount;

        if account.has_sufficient_funds(amount) {
            account.available = account
                .available
                .checked_sub(amount)
                .ok_or(EngineError::NumericUnderflow)?;

            account.held = account
                .held
                .checked_add(amount)
                .ok_or(EngineError::NumericOverflow)?;

            account.update_total();
            deposit.disputed = true;
        }

        Ok(())
    }

    pub(super) fn process_resolve(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get_mut(&transaction.tx) {
            Some(d) => d,
            None => return Ok(()),
        };

        if deposit.client_id != transaction.client || !deposit.disputed {
            return Ok(());
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Ok(()),
        };

        if account.locked {
            return Ok(());
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

        account.update_total();

        // Lets remove the resolved deposit to free memory, as it can't be disputed again
        self.deposits.remove(&transaction.tx);

        Ok(())
    }

    pub(super) fn process_chargeback(&mut self, transaction: Transaction) -> Result<()> {
        let deposit = match self.deposits.get(&transaction.tx) {
            Some(d) => d,
            None => return Ok(()),
        };

        if deposit.client_id != transaction.client || !deposit.disputed {
            return Ok(());
        }

        let account = match self.accounts.get_mut(&transaction.client) {
            Some(a) => a,
            None => return Ok(()),
        };

        if account.locked {
            return Ok(());
        }

        let amount = deposit.amount;
        let client_id = deposit.client_id;

        account.held = account
            .held
            .checked_sub(amount)
            .ok_or(EngineError::NumericUnderflow)?;

        account.update_total();
        account.locked = true;

        // Lets remove all deposits for this client since the account
        // is now locked and no further transactions can occur
        self.deposits.retain(|_, d| d.client_id != client_id);

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

    #[test]
    fn dispute_and_resolve_flow_moves_funds_between_available_and_held() {
        let mut processor = TransactionProcessor::new();

        processor.process(deposit(1, 1, "10.0")).unwrap();
        processor.process(deposit(1, 2, "3.0")).unwrap();
        processor.process(dispute(1, 1)).unwrap();

        {
            let account = processor.accounts.get(&1).unwrap();
            assert_eq!(account.available, decimal("3.0"));
            assert_eq!(account.held, decimal("10.0"));
            assert_eq!(account.total, decimal("13.0"));

            let deposit = processor.deposits.get(&1).unwrap();
            assert!(deposit.disputed);
        }

        processor.process(resolve(1, 1)).unwrap();
        let account = processor.accounts.get(&1).unwrap();

        assert_eq!(account.available, decimal("13.0"));
        assert_eq!(account.held, Decimal::ZERO);
        assert_eq!(account.total, decimal("13.0"));
    }

    #[test]
    fn chargeback_locks_account_and_clears_client_deposits() {
        let mut processor = TransactionProcessor::new();

        processor.process(deposit(1, 1, "8.0")).unwrap();
        processor.process(deposit(1, 2, "4.0")).unwrap();
        processor.process(dispute(1, 1)).unwrap();
        processor.process(chargeback(1, 1)).unwrap();

        let account = processor.accounts.get(&1).unwrap();

        assert_eq!(account.available, decimal("4.0"));
        assert_eq!(account.held, Decimal::ZERO);
        assert_eq!(account.total, decimal("4.0"));

        assert!(account.locked);
        assert!(processor.deposits.is_empty());

        // Further activity on a locked account should be ignored
        processor.process(deposit(1, 3, "2.0")).unwrap();
        let account_after = processor.accounts.get(&1).unwrap();

        assert_eq!(account_after.available, decimal("4.0"));
        assert_eq!(account_after.total, decimal("4.0"));
    }
}
