use crate::engine::transaction::Transaction;
use crate::Result;

use csv::{Reader, ReaderBuilder, Trim};
use log::{debug, error, info, warn};
use std::io::Read;

pub trait TransactionParser {
    fn parse<F>(self, handler: F) -> Result<()>
    where
        F: FnMut(Transaction) -> Result<()>;
}

pub struct TransactionParserCsv<R: Read> {
    reader: Reader<R>,
}

impl<R: Read> TransactionParserCsv<R> {
    pub fn new(reader: R) -> Self {
        let reader = ReaderBuilder::new().trim(Trim::All).from_reader(reader);

        Self { reader }
    }
}

impl<R: Read> TransactionParser for TransactionParserCsv<R> {
    fn parse<F>(mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(Transaction) -> Result<()>,
    {
        for (line_number, result) in self.reader.deserialize().enumerate() {
            let line_number = line_number + 2; // account for header
            let transaction: Transaction = match result {
                Ok(tx) => tx,
                Err(e) => {
                    error!(
                        "Failed to parse transaction: line={} error={}",
                        line_number, e
                    );
                    continue;
                }
            };

            debug!(
                "Processing transaction: line={} tx_id={} client_id={} tx_type={:?} amount={:?}",
                line_number,
                transaction.tx,
                transaction.client,
                transaction.tx_type,
                transaction.amount
            );

            if let Err(e) = handler(transaction.clone()) {
                warn!(
                    "Transaction failed: line={} tx_id={} client_id={} tx_type={:?} error={}",
                    line_number, transaction.tx, transaction.client, transaction.tx_type, e
                );
            } else {
                info!(
                    "Transaction processed: tx_id={} client_id={} tx_type={:?} amount={:?}",
                    transaction.tx, transaction.client, transaction.tx_type, transaction.amount
                );
            }
        }

        Ok(())
    }
}
