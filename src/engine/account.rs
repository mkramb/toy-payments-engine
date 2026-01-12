use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Account {
    pub client_id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    pub fn has_sufficient_funds(&self, amount: Decimal) -> bool {
        self.available >= amount
    }
}

/// Separates domain model from output format
#[derive(Serialize)]
pub struct AccountOutput {
    client: u16,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<&Account> for AccountOutput {
    fn from(account: &Account) -> Self {
        Self {
            client: account.client_id,
            available: format_decimal(account.available),
            held: format_decimal(account.held),
            total: format_decimal(account.total()),
            locked: account.locked,
        }
    }
}

fn format_decimal(value: Decimal) -> String {
    value.round_dp(4).normalize().to_string()
}
