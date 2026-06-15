use crate::consts::DATABASE_MEM_MAP;
use crate::data::{DataKey, StockData};
use heed::types::SerdeBincode;
use heed::{Database as DB, Env, EnvFlags, EnvOpenOptions, WithoutTls};

/// The database containing fetched [StockData] values.
#[derive(Clone)]
pub struct Database {
    database: DB<SerdeBincode<DataKey>, SerdeBincode<StockData>>,
    env: Env<WithoutTls>,
}

impl Database {
    /// Connects to the database, creating one if it does not exist yet.
    ///
    /// The database is stored in `./database.lmdb`.
    pub fn new() -> Self {
        let path = "./database.lmdb";

        if !std::fs::exists(path).expect("Failed to check database environment existence") {
            std::fs::create_dir_all(path).expect("Failed to create database environment");
        }

        let env = unsafe {
            EnvOpenOptions::new()
                .read_txn_without_tls()
                .map_size(DATABASE_MEM_MAP)
                .max_dbs(1)
                .max_readers(16)
                .flags(EnvFlags::empty())
                .open(path)
        }
        .expect("Failed to open database environment");

        let mut txn = env
            .write_txn()
            .expect("Failed to acquire read-write handle");

        let database = env
            .create_database(&mut txn, Some("database"))
            .expect("Failed to create database");

        txn.commit().expect("Failed to commit write handle");

        Self { database, env }
    }

    /// Sets a [StockData] value in the database using the specified [DataKey].
    pub fn set(&mut self, key: DataKey, data: StockData) {
        let mut txn = self
            .env
            .write_txn()
            .expect("Failed to acquire write handle");

        self.database
            .put(&mut txn, &key, &data)
            .expect("Failed to put data");

        txn.commit().expect("Failed to commit write handle");
    }

    /// Gets a [StockData] value from the database using the specified [DataKey].
    ///
    /// Returns `None` if the key does not exist.
    pub fn get(&self, key: &DataKey) -> Option<StockData> {
        let txn = self.env.read_txn().expect("Failed to acquire read handle");

        self.database.get(&txn, key).expect("Failed to get entry")
    }
}
