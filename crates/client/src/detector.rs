use crate::{
    commander::{Commander, Error as CommanderError},
    config::Config,
};
use fehler::{throw, throws};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    Commander(#[from] CommanderError),
    #[error("Cannot find the Anchor.toml file to locate the root folder")]
    BadWorkspace,
}

pub struct Detector {
    module_name: String,
}
impl Detector {
    pub fn new(module_name: String) -> Self {
        Self { module_name }
    }
    #[throws]
    pub async fn detect(&self) {
        let root = match Config::discover_root() {
            Ok(root) => root,
            Err(_) => throw!(Error::BadWorkspace),
        };
        let root_path = root.to_str().unwrap().to_string();
        let commander = Commander::with_root(root_path);
        commander
            .detect_program_client_lib_rs(&self.module_name)
            .await?;
    }
}
