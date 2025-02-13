#[cfg(feature = "sql")]
#[allow(dead_code)]
pub(crate) mod pgsql {
    use rio_rs::state::sql::SqlState;
    use sqlx::{any::AnyPoolOptions, Any, Pool};

    pub(crate) async fn pool(name: &str) -> Pool<Any> {
        let pool = AnyPoolOptions::new()
            .connect("postgres://test:test@localhost:15432/test")
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let sql = format!("DROP DATABASE IF EXISTS {name};");
        sqlx::query(&sql).execute(&mut conn).await.unwrap();

        let sql = format!("CREATE DATABASE {name} WITH OWNER=test;");
        sqlx::query(&sql).execute(&mut conn).await.unwrap();

        let new_conn_str = format!("postgres://test:test@localhost:15432/{name}");
        let pool = SqlState::pool().connect(&&new_conn_str).await.unwrap();
        pool
    }
}

#[allow(dead_code)]
#[cfg(feature = "sql")]
pub(crate) mod sqlite {
    use sqlx::{any::AnyPoolOptions, Any, Pool};

    pub(crate) async fn pool() -> Pool<Any> {
        let pool = AnyPoolOptions::new()
            .connect("sqlite://:memory:")
            .await
            .unwrap();
        pool
    }
}
