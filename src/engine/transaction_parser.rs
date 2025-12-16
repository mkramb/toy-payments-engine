use crate::engine::transaction::Transaction;
use crate::Result;

use csv::{Reader, ReaderBuilder, Trim};
use log::{error, warn};
use std::io::Read;

pub struct TransactionParserCsv<R: Read> {
    reader: Reader<R>,
}

impl<R: Read> TransactionParserCsv<R> {
    pub fn new(reader: R) -> Self {
        let reader = ReaderBuilder::new().trim(Trim::All).from_reader(reader);

        Self { reader }
    }

    pub fn parse<F>(mut self, mut handler: F) -> Result<()>
    where
        F: FnMut(Transaction) -> Result<()>,
    {
        for (line_number, result) in self.reader.deserialize().enumerate() {
            let line_number = line_number + 2; // account for header
            let transaction = match result {
                Ok(tx) => tx,
                Err(e) => {
                    error!("Dropped transaction on line {}: {}", line_number, e);
                    continue;
                }
            };

            if let Err(e) = handler(transaction) {
                warn!(
                    "Warning: Failed to process transaction on line {}: {}",
                    line_number, e
                );
            }
        }

        Ok(())
    }
}
