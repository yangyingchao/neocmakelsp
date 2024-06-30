use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct Config {
    pub scan_cmake_in_package: Option<bool>,
    pub semantic_token: Option<bool>,
}

impl Config {
    pub fn is_scan_cmake_in_package(&self) -> bool {
        self.scan_cmake_in_package.unwrap_or(true)
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
