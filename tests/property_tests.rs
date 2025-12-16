use payments_engine::{Transaction, TransactionType};

use proptest::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

fn transaction_strategy() -> impl Strategy<Value = Transaction> {
    (
        prop_oneof![
            9 => 1u16..100,    // 90%: constrained range for client collisions
            1 => any::<u16>(), // 10%: edge case values (0, 65535, etc.)
        ],
        prop_oneof![
            9 => 1u32..1000,   // 90%: constrained range for tx collisions
            1 => any::<u32>(), // 10%: edge case values
        ],
        prop_oneof![
            Just(TransactionType::Deposit),
            Just(TransactionType::Withdrawal),
            Just(TransactionType::Dispute),
            Just(TransactionType::Resolve),
            Just(TransactionType::Chargeback),
        ],
        prop::option::of(1i64..1_000_000), // amount in cents
    )
        .prop_map(|(client, tx, tx_type, amount_cents)| {
            let amount = amount_cents.map(|c| Decimal::new(c, 2));
            Transaction {
                client,
                tx_type,
                tx,
                amount,
            }
        })
}

mod invariants {
    use super::*;
    use payments_engine::{Format, PaymentsEngine};
    use std::io::Cursor;

    fn process_transactions(transactions: Vec<Transaction>) -> PaymentsEngine {
        let mut engine = PaymentsEngine::new();
        let mut csv_content = String::from("type,client,tx,amount\n");

        for tx in transactions {
            let tx_type = match tx.tx_type {
                TransactionType::Deposit => "deposit",
                TransactionType::Withdrawal => "withdrawal",
                TransactionType::Dispute => "dispute",
                TransactionType::Resolve => "resolve",
                TransactionType::Chargeback => "chargeback",
            };

            let amount = tx.amount.map(|a| a.to_string()).unwrap_or_else(String::new);
            csv_content.push_str(&format!("{},{},{},{}\n", tx_type, tx.client, tx.tx, amount));
        }

        let reader = Cursor::new(csv_content);
        let _ = engine.process_reader(reader, Format::Csv);
        engine
    }

    fn get_accounts(engine: &PaymentsEngine) -> HashMap<u16, (Decimal, Decimal, Decimal, bool)> {
        let mut output = Vec::new();
        engine.export_accounts(&mut output, Format::Csv).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let mut reader = csv::Reader::from_reader(output_str.as_bytes());

        reader
            .records()
            .filter_map(|r| r.ok())
            .filter_map(|record| {
                let client: u16 = record.get(0)?.parse().ok()?;
                let available: Decimal = record.get(1)?.parse().ok()?;
                let held: Decimal = record.get(2)?.parse().ok()?;
                let total: Decimal = record.get(3)?.parse().ok()?;
                let locked: bool = record.get(4)?.parse().ok()?;
                Some((client, (available, held, total, locked)))
            })
            .collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        #[test]
        fn total_always_equals_available_plus_held(
            transactions in prop::collection::vec(transaction_strategy(), 0..100)
        ) {
            let engine = process_transactions(transactions);
            let accounts = get_accounts(&engine);

            for (_client, (available, held, total, _locked)) in accounts {
                prop_assert_eq!(
                    total,
                    available + held,
                    "total ({}) != available ({}) + held ({})",
                    total,
                    available,
                    held
                );
            }
        }

        #[test]
        fn balances_never_go_negative(
            transactions in prop::collection::vec(transaction_strategy(), 0..100)
        ) {
            let engine = process_transactions(transactions);
            let accounts = get_accounts(&engine);

            for (client, (available, held, total, _locked)) in accounts {
                prop_assert!(
                    available >= Decimal::ZERO,
                    "Client {} has negative available: {}",
                    client,
                    available
                );
                prop_assert!(
                    held >= Decimal::ZERO,
                    "Client {} has negative held: {}",
                    client,
                    held
                );
                prop_assert!(
                    total >= Decimal::ZERO,
                    "Client {} has negative total: {}",
                    client,
                    total
                );
            }
        }

        #[test]
        fn locked_accounts_balances_never_change(
            base_txs in prop::collection::vec(transaction_strategy(), 1..50),
            extra_txs in prop::collection::vec(transaction_strategy(), 1..50)
        ) {
            let engine_before = process_transactions(base_txs.clone());
            let accounts_before = get_accounts(&engine_before);

            let locked_snapshots: HashMap<_, _> = accounts_before
                .iter()
                .filter(|(_, (_, _, _, locked))| *locked)
                .map(|(id, snapshot)| (*id, *snapshot))
                .collect();

            let mut all_txs = base_txs;
            all_txs.extend(extra_txs);

            let engine_after = process_transactions(all_txs);
            let accounts_after = get_accounts(&engine_after);

            for (client_id, (avail, held, total, _)) in locked_snapshots {
                if let Some((new_avail, new_held, new_total, new_locked)) =
                    accounts_after.get(&client_id)
                {
                    prop_assert!(
                        *new_locked,
                        "Client {} was locked but is now unlocked",
                        client_id
                    );
                    prop_assert_eq!(
                        *new_avail, avail,
                        "Locked client {} available changed from {} to {}",
                        client_id, avail, new_avail
                    );
                    prop_assert_eq!(
                        *new_held, held,
                        "Locked client {} held changed from {} to {}",
                        client_id, held, new_held
                    );
                    prop_assert_eq!(
                        *new_total, total,
                        "Locked client {} total changed from {} to {}",
                        client_id, total, new_total
                    );
                }
            }
        }

        #[test]
        fn processing_never_panics(
            transactions in prop::collection::vec(transaction_strategy(), 0..200)
        ) {
            let _engine = process_transactions(transactions);
            // If we get here without panicking, the test passes
        }
    }
}
