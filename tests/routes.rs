use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;

use uuid::Uuid;
use z2p::{
    configuration::{self, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

static LOCAL_HOST: &str = "127.0.0.1:0";

pub struct TestApp {
    pub addr: String,
    pub db_pool: PgPool,
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let res = client
        .get(&format!("{}/health_check", &app.addr))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=joe%20smith&email=joe.smith%40gmail.com";
    let res = client
        .post(&format!("{}/subscriptions", &app.addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(200, res.status().as_u16());
    assert_eq!(saved.email, "joe.smith@gmail.com");
    assert_eq!(saved.name, "joe smith");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=john%20doe", "missing email"),
        ("email=john.doe%40gmail.com", "missing name"),
        ("", "missing both name and email"),
        ("name=&email=john@gmail.com", "empty name"),
    ];

    for (invalid_body, err_msg) in test_cases {
        let res = client
            .post(&format!("{}/subscriptions", &app.addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");
        assert_eq!(
            400,
            res.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            err_msg
        )
    }
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind(LOCAL_HOST).expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{}", port);

    let mut config = configuration::get_configuration().expect("Failed to read config");
    config.database.database_name = Uuid::new_v4().to_string();

    let conn_pool = configure_db(&config.database).await;

    let server = z2p::startup::run(listener, conn_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        addr,
        db_pool: conn_pool,
    }
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut conn = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Couldn't connect to db");

    conn.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let conn_pool = PgPool::connect_lazy_with(config.without_db());

    sqlx::migrate!("./migrations")
        .run(&conn_pool)
        .await
        .expect("Failed to migrate db");

    conn_pool
}
