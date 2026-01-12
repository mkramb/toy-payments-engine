use crate::engine::account_writer::{AccountWriter, AccountWriterCsv};
use crate::engine::transaction_parser::{TransactionParser, TransactionParserCsv};
use crate::engine::{transaction::Transaction, transaction_processor::TransactionProcessor};
use crate::Result;

use std::io::{self, Write};

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Csv,
}

pub struct PaymentsEngine {
    processor: TransactionProcessor,
}

impl PaymentsEngine {
    pub fn new() -> Self {
        Self {
            processor: TransactionProcessor::new(),
        }
    }

    pub fn process_reader<R: io::Read>(&mut self, reader: R, format: Format) -> Result<()> {
        match format {
            Format::Csv => {
                let parser = TransactionParserCsv::new(reader);
                parser.parse(|transaction: Transaction| self.processor.process(transaction))?;
            }
        }

        Ok(())
    }

    pub fn export_accounts<W: Write>(&self, writer: W, format: Format) -> Result<()> {
        match format {
            Format::Csv => {
                let mut exporter = AccountWriterCsv::new(writer);
                exporter.write_accounts(self.processor.accounts().values())?;
            }
        }

        Ok(())
    }
}

impl Default for PaymentsEngine {
    fn default() -> Self {
        Self::new()
    }
}
