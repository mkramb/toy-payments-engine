use crate::engine::account::{Account, AccountOutput};
use crate::Result;

use csv::Writer;
use std::io::Write;

pub trait AccountWriter {
    fn write_accounts<'a, I>(&mut self, accounts: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a Account>;
}

pub struct AccountWriterCsv<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> AccountWriterCsv<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: Writer::from_writer(inner),
        }
    }
}

impl<W: Write> AccountWriter for AccountWriterCsv<W> {
    fn write_accounts<'a, I>(&mut self, accounts: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a Account>,
    {
        for account in accounts {
            self.writer.serialize(AccountOutput::from(account))?;
        }
        self.writer.flush()?;
        Ok(())
    }
}
