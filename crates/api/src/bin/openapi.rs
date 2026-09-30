//! Print the OpenAPI spec to stdout. Touches no database and reads no
//! environment, so CI can run it on a bare checkout:
//!
//! ```bash
//! cargo run -p api --bin openapi > docs/openapi.json
//! git diff --exit-code docs/openapi.json   # fails if the committed spec drifted
//! ```

fn main() -> anyhow::Result<()> {
    println!("{}", api::http::routes::openapi().to_pretty_json()?);

    Ok(())
}
