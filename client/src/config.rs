

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CbakConfig {
    pub global: _GlobalConfig,
    pub watch: Vec<_DirConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _CbakConfig {
    pub global: _GlobalConfig,
    pub watch: Option<Vec<_DirConfig>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _GlobalConfig {
    pub ignore: Vec<String>,
    pub poll_interval: i32,
    pub write_delay: i32,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct _DirConfig {
    pub directory: String,
    pub ignore: Vec<String>,
    pub poll_interval: Option<i32>,
    pub write_delay: Option<i32>,
    pub name: String,
}

impl CbakConfig {
    pub fn new(data: &str) -> Self {
        let config: _CbakConfig = toml::from_str(data).unwrap();
        Self {
            global: config.global,
            watch: if config.watch.is_none() {
                vec![]
            } else {
                config.watch.unwrap()
            },
        }
    }
}
