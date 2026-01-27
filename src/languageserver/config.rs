use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Config {
    pub scan_cmake_in_package: Option<bool>,
    pub semantic_token: Option<bool>,
}

impl Config {
    pub fn is_scan_cmake_in_package(&self) -> bool {
        self.scan_cmake_in_package.unwrap_or(true)
    }

    pub fn enable_semantic_token(&self) -> bool {
        self.semantic_token.unwrap_or(false)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scan_cmake_in_package: Some(true),
            semantic_token: Some(false),
        }
    }
}
