use std::io::{Read, Write};
use std::path::PathBuf;
use std::{fs, path::Path};

use fancy_regex::Regex;
use serde::{Deserialize, Serialize};

// Any struct prefixed with an _ is what the configuration is seralized into,
// the "normal" structs are what are used by the client, the _ structs are converted into the "normal" ones

#[derive(Clone, Serialize, Deserialize, Debug)]
struct _CbakConfig {
    global: _GlobalConfig,
    watch: Option<Vec<_DirConfig>>,
}

#[derive(Clone, Debug)]
pub struct CbakConfig {
    pub global: GlobalConfig,
    pub watch: Vec<DirConfig>,
    pub config_file_path: PathBuf,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct _GlobalConfig {
    ignore: Vec<String>,
    poll_interval: i32,
    write_delay: i32,
}

#[derive(Clone, Debug)]
pub struct GlobalConfig {
    pub ignore: Vec<Regex>,
    pub poll_interval: i32,
    pub write_delay: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct _DirConfig {
    directory: String,
    ignore: Vec<String>,
    poll_interval: Option<i32>,
    write_delay: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct DirConfig {
    pub directory: String,
    pub ignore: Vec<Regex>,
    pub poll_interval: i32,
    pub write_delay: i32,
}

impl CbakConfig {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        if !Path::new("config.toml").exists() {
            let mut file = fs::File::create("config.toml").unwrap();
            write!(
                file,
                "[global]
ignore = [\".git\\\\\\\\\", \"\\\\\\\\.git\", \"/.git\", \".git/\"]
poll_interval = 30
write_delay = 30
"
            )
            .unwrap();
        }
        let mut file = fs::File::open("config.toml")?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let config: _CbakConfig = toml::from_slice(buf.as_slice())?;

        Ok(Self {
            global: GlobalConfig {
                ignore: config
                    .global
                    .ignore
                    .iter()
                    .map(|f| Regex::new(f).unwrap())
                    .collect(),
                poll_interval: config.global.poll_interval,
                write_delay: config.global.write_delay,
            },
            config_file_path: fs::canonicalize("config.toml")?,
            watch: if config.watch.is_some() {
                config
                    .watch
                    .unwrap()
                    .iter()
                    .map(|i| DirConfig {
                        directory: i.directory.clone(),
                        ignore: i.ignore.iter().map(|f| Regex::new(f).unwrap()).collect(),
                        poll_interval: i.poll_interval.unwrap_or(config.global.poll_interval),
                        write_delay: i.write_delay.unwrap_or(config.global.write_delay),
                    })
                    .collect()
            } else {
                vec![]
            },
        })
    }
}
