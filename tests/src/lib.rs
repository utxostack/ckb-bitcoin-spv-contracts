use ckb_testtool::{
    ckb_error::Error,
    ckb_types::{
        bytes::Bytes,
        core::{Cycle, TransactionView},
    },
    context::Context,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(test)]
mod contracts;
#[cfg(test)]
pub(crate) mod utilities;

pub mod prelude {
    use ckb_testtool::{
        ckb_error::Error,
        ckb_types::core::{Cycle, TransactionView},
    };

    pub const MAX_CYCLES: u64 = 10_000_000;
    pub const SPV_CELL_CAP: u64 = 500;
    pub const SPV_HEADERS_GROUP_SIZE: usize = 20; // Speed up to save time.

    // This helper method runs Context::verify_tx, but in case error happens,
    // it also dumps current transaction to failed_txs folder.
    pub trait ContextExt {
        fn should_be_passed(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, Error>;
        fn should_be_failed(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, Error>;
    }
}

// The exact same Loader code from capsule's template, except that
// now we use MODE as the environment variable
const TEST_ENV_VAR: &str = "MODE";

pub enum TestEnv {
    Debug,
    Release,
}

impl FromStr for TestEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(TestEnv::Debug),
            "release" => Ok(TestEnv::Release),
            _ => Err("no match"),
        }
    }
}

pub struct Loader(PathBuf);

impl Default for Loader {
    fn default() -> Self {
        let test_env = match env::var(TEST_ENV_VAR) {
            Ok(val) => val.parse().expect("test env"),
            Err(_) => TestEnv::Release,
        };
        Self::with_test_env(test_env)
    }
}

impl Loader {
    fn with_test_env(env: TestEnv) -> Self {
        let load_prefix = match env {
            TestEnv::Debug => "debug",
            TestEnv::Release => "release",
        };
        let mut base_path = match env::var("TOP") {
            Ok(val) => {
                let mut base_path: PathBuf = val.into();
                base_path.push("build");
                base_path
            }
            Err(_) => {
                let mut base_path = PathBuf::new();
                // cargo may use a different cwd when running tests, for example:
                // when running debug in vscode, it will use workspace root as cwd by default,
                // when running test by `cargo test`, it will use tests directory as cwd,
                // so we need a fallback path
                base_path.push("build");
                if !base_path.exists() {
                    base_path.pop();
                    base_path.push("..");
                    base_path.push("build");
                }
                base_path
            }
        };

        base_path.push(load_prefix);
        Loader(base_path)
    }

    pub fn load_binary(&self, name: &str) -> Bytes {
        let mut path = self.0.clone();
        path.push(name);
        let result = fs::read(&path);
        if result.is_err() {
            panic!("Binary {:?} is missing!", path);
        }
        result.unwrap().into()
    }
}

impl prelude::ContextExt for Context {
    fn should_be_passed(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, Error> {
        let result = self.verify_tx(tx, max_cycles);
        if let Err(err) = result {
            let mut path = env::current_dir().expect("current dir");
            path.push("failed_txs");
            std::fs::create_dir_all(&path).expect("create failed_txs dir");
            let mock_tx = self.dump_tx(tx).expect("dump failed tx");
            let json = serde_json::to_string_pretty(&mock_tx).expect("json");
            path.push(format!("0x{:x}.json", tx.hash()));
            println!("Failed tx written to {:?}", path);
            std::fs::write(path, json).expect("write");
            panic!("should be passed, but failed since {err}");
        }
        result
    }

    fn should_be_failed(&self, tx: &TransactionView, max_cycles: u64) -> Result<Cycle, Error> {
        let result = self.verify_tx(tx, max_cycles);
        if result.is_ok() {
            let mut path = env::current_dir().expect("current dir");
            path.push("failed_txs");
            std::fs::create_dir_all(&path).expect("create failed_txs dir");
            let mock_tx = self.dump_tx(tx).expect("dump failed tx");
            let json = serde_json::to_string_pretty(&mock_tx).expect("json");
            path.push(format!("0x{:x}.json", tx.hash()));
            println!("Failed tx written to {:?}", path);
            std::fs::write(path, json).expect("write");
            panic!("should be failed");
        }
        result
    }
}
