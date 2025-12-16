use crate::engine::account::Account;
use crate::Result;

use csv::Writer;
use std::io::Write;

pub struct AccountWriterCsv<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> AccountWriterCsv<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: Writer::from_writer(inner),
        }
    }

    pub fn write_accounts<'a, I>(&mut self, accounts: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a Account>,
    {
        let mut accounts: Vec<_> = accounts.into_iter().collect();
        accounts.sort_by_key(|a| a.client_id);

        for account in accounts {
            self.writer.serialize(account)?;
        }
        self.writer.flush()?;
        Ok(())
    }
}
