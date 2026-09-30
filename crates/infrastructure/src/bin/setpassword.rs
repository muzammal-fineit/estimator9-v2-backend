//! Sets a user's password.
//!
//! ```bash
//! cargo run -p infrastructure --bin setpassword -- akmal@tech.fineit.io
//! ```
//!
//! This solves the bootstrap problem: the seeded accounts have no usable
//! password, and there is no endpoint to create one until an operator can log
//! in. It is the same tool in production, where it is how the first superadmin
//! becomes able to sign in.
//!
//! The password is read from stdin rather than taken as an argument, so it does
//! not land in shell history or in `ps` output for every user on the box. It is
//! still echoed to the terminal — shoulder-surfing is out of scope for a
//! command run once at install time.

use std::io::Write;

use anyhow::{Context, bail};
use domain::identity::{Email, Password, PasswordHasher, UserRepository};
use infrastructure::persistence::identity::PgUserRepository;
use infrastructure::security::Argon2Hasher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let Some(raw_email) = std::env::args().nth(1) else {
        bail!("usage: setpassword <email>");
    };

    let email = Email::parse(raw_email).context("that is not a valid email address")?;

    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let pool = infrastructure::connect(&url, 1)
        .await
        .context("could not connect to the database")?;

    let users = PgUserRepository::new(pool);

    // Look the account up before asking for a password, so a typo in the email
    // is reported immediately rather than after the operator has typed one.
    let user = users
        .find_by_email(&email)
        .await?
        .with_context(|| format!("no account with the email {email}"))?;

    let password =
        Password::parse(prompt("New password: ")?).context("password rejected by policy")?;

    let confirmation =
        Password::parse(prompt("Repeat password: ")?).context("password rejected by policy")?;

    if password != confirmation {
        bail!("the two passwords do not match");
    }

    let hash = Argon2Hasher::new().hash(&password).await?;
    users.set_password(user.id(), &hash).await?;

    println!(
        "password set for {} (id {}); any lockout has been cleared",
        user.email(),
        user.id()
    );

    Ok(())
}

fn prompt(label: &str) -> anyhow::Result<String> {
    print!("{label}");
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("could not read from stdin")?;

    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}
