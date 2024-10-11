pub trait SqlMigrations {
    fn queries() -> Vec<String>;
}
