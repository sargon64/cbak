use std::io::{Read, Write};
use std::{fs, path::Path};

use regex::RegexSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _CbakConfig {
    pub global: _GlobalConfig,
    pub watch: Vec<_DirConfig>,
}

#[derive(Clone, Debug)]
pub struct CbakConfig {
    pub global: GlobalConfig,
    pub watch: Vec<DirConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _GlobalConfig {
    pub ignore: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GlobalConfig {
    pub ignore: RegexSet,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct _DirConfig {
    pub directory: String,
    pub ignore: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DirConfig {
    pub directory: String,
    pub ignore: RegexSet,
}

impl CbakConfig {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        if !Path::new("config.toml").exists() {
            let mut file = fs::File::create("config.toml").unwrap();
            write!(
                file,
                "[global]
ignore = [ '.git\\\\' ]

# a watch enrty, ignore is a regex of files to be ignored. you can have more then one regex
[[watch]]
directory = '' 
ignore = [] 

[[watch]]
directory = ''
ignore = []"
            )
            .unwrap();
            panic!("no config found, generating")
        }
        let mut file = fs::File::open("config.toml")?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let config: _CbakConfig = toml::from_slice(buf.as_slice())?;

        Ok(Self {
            global: GlobalConfig {
                ignore: RegexSet::new(config.global.ignore.as_slice())?,
            },
            watch: config
                .watch
                .iter()
                .map(|i| DirConfig {
                    directory: i.directory.clone(),
                    ignore: RegexSet::new(
                        i.ignore
                            .iter()
                            .chain(config.global.ignore.iter())
                            .map(|i| i.to_owned())
                            .collect::<Vec<String>>()
                            .as_slice(),
                    )
                    .unwrap(),
                })
                .collect(),
        })
    }
}
