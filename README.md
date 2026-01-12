# Toy Payments Engine

A toy payment engine built in Rust.

## Example Usage

```
cargo build --release
cargo run -- transactions.csv > accounts.csv
```

Development

```
make validate
make help
```

## Architecture

- **Streaming parsing**
- **Optimized lookups and transaction processing** - excluding chargeback is O(n) due to deposit cleanup
- **Memory-conscious** — resolved deposits are removed & chargeback deposits are cleaned up

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────┐
│  CSV Input  │────▶│  PaymentsEngine  │────▶│  CSV Output  │
└─────────────┘     └──────────────────┘     └──────────────┘
                            │
                    ┌───────▼───────┐
                    │  Processor    │
                    │  - accounts   │ BTreeMap<u16, Account>
                    │  - deposits   │ BTreeMap<u32, DepositRecord>
                    └───────────────┘
```

### Validation / Tests

- **Unit tests** — Cover core transaction processing logic
- **E2E tests** — Full pipeline validation using fixture CSV files
- **Property tests** — Catching edge cases humans wouldn't think to test
- **Manual testing** - See prepared examples CSV files

### Potential Improvements

- **Partitioning** - Process independent clients concurrently
- **Message queue** - Durability, natural concurrency, and backpressure handling
- **Event sourcing** - Maintain complete transaction history for audit trails and dispute resolution
- **Metrics & observability** - Transaction counts, processing latency, error rates and dispute patterns

### Integration As Library

The engine is structured as a library with a thin CLI wrapper.

<details>
<summary>Example of REST API integration</summary>

```rust
use axum::{extract::State, http::StatusCode, Router};
use payments_engine::{PaymentsEngine, Format};

use std::io::Cursor;
use std::sync::{Arc, Mutex};

type SharedEngine = Arc<Mutex<PaymentsEngine>>;

async fn process_transactions(
    State(engine): State<SharedEngine>,
    body: String,  // CSV payload from request
) -> Result<StatusCode, String> {
    let mut engine = engine.lock().unwrap();

    engine
        .process_reader(Cursor::new(body), Format::Csv)
        .map_err(|e| e.to_string())?;

    Ok(StatusCode::OK)
}
```

</details>

## Use of AI

- generated example CSV files for manually validation
- for bootstrapping tests, specifically the props tests setup
- to generate base README file
