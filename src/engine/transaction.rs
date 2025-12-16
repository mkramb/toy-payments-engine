use rust_decimal::Decimal;
use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Transaction {
    pub client: u16,
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub tx: u32,
    #[serde(default, deserialize_with = "deserialize_amount")]
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub(crate) struct DepositRecord {
    pub client_id: u16,
    pub amount: Decimal,
    pub disputed: bool,
}

impl DepositRecord {
    pub fn new(client_id: u16, amount: Decimal) -> Self {
        Self {
            client_id,
            amount,
            disputed: false,
        }
    }
}

fn deserialize_amount<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let amount = Option::<Decimal>::deserialize(deserializer)?;

    if let Some(value) = amount {
        if value.scale() > 4 {
            return Err(DeError::custom("amount precision exceeds 4 decimal places"));
        }
        return Ok(Some(value));
    }

    Ok(None)
}
