use crate::engine::account::Account;
use crate::engine::transaction::{DepositRecord, Transaction, TransactionType};
use crate::Result;

use std::collections::BTreeMap;

/// Processes financial transactions maintaining account state.
///
/// # Thread Safety
///
/// This processor requires exclusive (`&mut`) access for all operations.
/// For concurrent environments (e.g., HTTP streaming), wrap in `Arc<Mutex<_>>`
/// or use an actor pattern where a single task owns the processor.
pub struct TransactionProcessor {
    accounts: BTreeMap<u16, Account>,
    deposits: BTreeMap<u32, DepositRecord>,
}

#[path = "transaction_processor_deposits.rs"]
mod transaction_processor_deposits;

#[path = "transaction_processor_disputes.rs"]
mod transaction_processor_disputes;

impl TransactionProcessor {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            deposits: BTreeMap::new(),
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

    pub fn accounts(&self) -> &BTreeMap<u16, Account> {
        &self.accounts
    }
}

impl Default for TransactionProcessor {
    fn default() -> Self {
        Self::new()
    }
}
