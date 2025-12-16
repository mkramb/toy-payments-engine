use crate::engine::account::Account;
use crate::engine::transaction::{DepositRecord, Transaction, TransactionType};
use crate::Result;

use std::collections::HashMap;

pub struct TransactionProcessor {
    accounts: HashMap<u16, Account>,
    deposits: HashMap<u32, DepositRecord>,
}

#[path = "transaction_processor_deposits.rs"]
mod transaction_processor_deposits;

#[path = "transaction_processor_disputes.rs"]
mod transaction_processor_disputes;

impl TransactionProcessor {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::default(),
            deposits: HashMap::default(),
        }
    }

    pub fn process(&mut self, transaction: Transaction) -> Result<()> {
        match transaction.tx_type {
            TransactionType::Deposit => self.process_deposit(transaction),
            TransactionType::Withdrawal => self.process_withdrawal(transaction),
            TransactionType::Dispute => self.process_dispute(transaction),
            TransactionType::Resolve => self.process_resolve(transaction),
            TransactionType::Chargeback => self.process_chargeback(transaction),
        }
    }

    pub fn accounts(&self) -> &HashMap<u16, Account> {
        &self.accounts
    }
}

impl Default for TransactionProcessor {
    fn default() -> Self {
        Self::new()
    }
}
