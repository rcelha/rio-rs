#[cfg(feature = "postgres")]
#[allow(dead_code)]
pub(crate) mod pgsql {
    use rio_rs::state::postgres::PostgresState;
    use sqlx::{postgres::PgPoolOptions, PgPool};

    pub(crate) async fn pool(name: &str) -> PgPool {
        let pool = PostgresState::pool()
            .connect("postgres://test:test@localhost:15432/test")
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let sql = format!("DROP DATABASE IF EXISTS {name};");
        sqlx::query(&sql).execute(&mut conn).await.unwrap();

        let sql = format!("CREATE DATABASE {name} WITH OWNER=test;");
        sqlx::query(&sql).execute(&mut conn).await.unwrap();

        let new_conn_str = format!("postgres://test:test@localhost:15432/{name}");
        let pool = PgPoolOptions::new().connect(&&new_conn_str).await.unwrap();
        pool
    }
}

#[allow(dead_code)]
#[cfg(feature = "sql")]
pub(crate) mod sqlite {
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

    pub(crate) async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite://:memory:")
            .await
            .unwrap();
        pool
    }
}
