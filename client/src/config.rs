use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _CbakConfig {
    pub global: _GlobalConfig,
    pub watch: Vec<_DirConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _GlobalConfig {
    pub ignore: Vec<String>,
    pub poll_interval: i32,
    pub write_delay: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _DirConfig {
    pub directory: String,
    pub ignore: Vec<String>,
    pub poll_interval: Option<i32>,
    pub write_delay: Option<i32>,
    pub name: String
}