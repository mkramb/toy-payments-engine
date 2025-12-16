use payments_engine::{Format, PaymentsEngine};

use rust_decimal::Decimal;
use serde::Deserialize;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize, PartialEq)]
struct TestAccount {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

fn run_engine_on_fixture(path: &str) -> payments_engine::Result<String> {
    let mut engine = PaymentsEngine::new();
    let file = File::open(Path::new(path))?;

    let reader = BufReader::new(file);
    engine.process_reader(reader, Format::Csv)?;

    let mut output = Vec::new();
    engine.export_accounts(&mut output, Format::Csv)?;

    Ok(String::from_utf8(output).expect("accounts output should be UTF-8"))
}

fn parse_accounts(csv_output: &str) -> Result<Vec<TestAccount>, csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_output.as_bytes());
    reader.deserialize().collect()
}

#[test]
fn sample_transactions_end_to_end() -> TestResult {
    let output = run_engine_on_fixture("examples/sample_transactions.csv")?;
    let accounts = parse_accounts(&output)?;

    assert_eq!(
        accounts,
        vec![
            TestAccount {
                client: 1,
                available: decimal("1.5"),
                held: Decimal::ZERO,
                total: decimal("1.5"),
                locked: false,
            },
            TestAccount {
                client: 2,
                available: decimal("2.0"),
                held: Decimal::ZERO,
                total: decimal("2.0"),
                locked: false,
            }
        ]
    );

    Ok(())
}

#[test]
fn all_operations_end_to_end() -> TestResult {
    let output = run_engine_on_fixture("examples/all_operations.csv")?;
    let accounts = parse_accounts(&output)?;

    assert_eq!(
        accounts,
        vec![
            TestAccount {
                client: 1,
                available: decimal("70.0"),
                held: Decimal::ZERO,
                total: decimal("70.0"),
                locked: true,
            },
            TestAccount {
                client: 2,
                available: decimal("150.0"),
                held: Decimal::ZERO,
                total: decimal("150.0"),
                locked: false,
            }
        ]
    );

    Ok(())
}

#[test]
fn mixed_valid_and_invalid_transactions_end_to_end() -> TestResult {
    let output = run_engine_on_fixture("examples/mixed_valid_invalid.csv")?;
    let accounts = parse_accounts(&output)?;

    assert_eq!(
        accounts,
        vec![
            TestAccount {
                client: 1,
                available: decimal("5.0"),
                held: Decimal::ZERO,
                total: decimal("5.0"),
                locked: true,
            },
            TestAccount {
                client: 2,
                available: decimal("40.0"),
                held: Decimal::ZERO,
                total: decimal("40.0"),
                locked: false,
            }
        ]
    );

    Ok(())
}
