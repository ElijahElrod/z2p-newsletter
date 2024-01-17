//! main.rs

use sqlx::Connection;
use sqlx::PgConnection;
use sqlx::PgPool;
use std::net::TcpListener;
use z2p::configuration;
use z2p::startup;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = configuration::get_configuration().expect("Failed to read config");
    let conn_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let addr = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(addr)?;
    startup::run(listener, conn_pool)?.await
}
