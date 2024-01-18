use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;
use z2p::configuration;
use z2p::startup;
use z2p::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("z2p".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = configuration::get_configuration().expect("Failed to read config");
    let conn_pool = PgPool::connect_lazy_with(&config.database.with_db())
        .expect("Failed to connect to Postgres.");

    let addr = format!("{}:{}", config.application.host, config.application.port);
    let listener = TcpListener::bind(addr)?;
    startup::run(listener, conn_pool)?.await?;
    Ok(())
}
