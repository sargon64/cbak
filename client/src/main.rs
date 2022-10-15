use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    vec, task::Waker,
};

use clap::{arg, command, value_parser, Command, ArgAction};
use cli_table::{format::Justify, print_stdout, Cell, CellStruct, Style, Table};
use interprocess::local_socket::{LocalSocketStream, NameTypeSupport};

mod config;

// #[derive(PartialEq, Clone)]
// struct ConfigOptionScopes {
//     Global: GlobalConfigOptions,
//     Watched: WatchedConfigOptions
// }

// #[derive(PartialEq, Clone)]
// struct GlobalConfigOptions {
//     PollInterval: i32,
//     WriteDelay: i32,
//     Ignore: Vec<String>
// }

// #[derive(PartialEq, Clone)]
// struct WatchedConfigOptions {
//     PollInterval: i32,
//     WriteDelay: i32,
//     Ignore: Vec<String>
// }

//#[derive(Subcommand, PartialEq)]
//#[derive(PartialEq, Clone, Subcommand)]
//enum ConfigOptionScopes {
//    Global {
//        #[command(subcommand, name = "key")]
//        key: GlobalConfigOptions
//    },
//    Watched
//}

//#[derive(Subcommand, PartialEq)]
//#[derive(PartialEq, Clone, Subcommand)]
//enum GlobalConfigOptions {
//    PollInterval {
//        value: i32
//    },
//    WriteDelay {
//        value: i32
//   },
//    Ignore {
//        value: Vec<String>
//    }
//}

fn main() {
    let matches = command!()
        .subcommand(
            Command::new("watch")
                .about("Add a directory to the watchlist")
                .arg(
                    arg!(<DIRECTORY> "Directory to watch")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(<NAME> "Name of the watch entry")
                        .required(true)
                        .value_parser(value_parser!(String)),
                )
                .arg(
                    arg!(-p --"poll-interval" <INTERVAL> "Frequency to poll directory")
                        .required(false)
                        .value_parser(value_parser!(i32)),
                )
                .arg(
                    arg!(-w --"write-delay" <DELAY> "Delay to wait for write completion")
                        .required(false)
                        .value_parser(value_parser!(i32)),
                )
                .arg(arg!([IGNORE]... "Set of regexs of directorys to ingore").required(false)),
        )
        .subcommand(
            Command::new("unwatch")
                .about("Remove a directory from the watchlist")
                .arg(
                    arg!(<NAME> "Name of entry to remove")
                        .required(true)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(Command::new("list").about("Print the watchlist"))
        .subcommand(Command::new("reload").about("Reloads the configuration & restarts all of the worker threads"))
        .subcommand(
            Command::new("edit-config")
                .about("Edit the configuration")
                .arg(
                    arg!(<SCOPE> "Configuration scope")
                        .required(true)
                        .value_parser(value_parser!(String)),
                )
                .arg(arg!([KEY] "Configuration key").required(false))
                .arg(arg!([VALUE]... "Configuration value").required(false))
                .arg(arg!( --"no-reload" "Don't reload daemon").required(false).action(ArgAction::SetTrue)),
        )
        .get_matches();

    let sock_name = {
        use NameTypeSupport::*;
        match NameTypeSupport::query() {
            OnlyPaths => "/tmp/cbak.sock",
            OnlyNamespaced | Both => "@cbak.sock",
        }
    };

    
    match matches.subcommand() {
        Some(("reload", _)) => {
            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            conn.get_mut()
                .write_all(&[0b0000_0010, 0xA])
                .expect("write failure");
       }
        Some(("watch", args)) => {
            let directory = args.get_one::<String>("DIRECTORY").unwrap();
            let name = args.get_one::<String>("NAME").unwrap();
            let poll_interval = args.get_one::<i32>("poll-interval");
            let write_delay = args.get_one::<i32>("write-delay");
            let ignore = args.get_many::<String>("IGNORE");

            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            let mut buf = String::new();
            conn.get_mut().write_all(&[0b0000_0100, 0xA]).unwrap();
            conn.read_line(&mut buf).unwrap();
            buf.pop();

            let conf_file_path = buf;
            let mut conf_file =
                fs::File::open(&conf_file_path).expect("Failed to open config file");

            let mut buf = String::new();
            conf_file
                .read_to_string(&mut buf)
                .expect("Failed to open config file");
            let mut conf = config::CbakConfig::new(&buf);

            conf.watch.push(config::_DirConfig {
                directory: fs::canonicalize(directory)
                    .expect("Not a valid directory")
                    .to_str()
                    .unwrap()
                    .to_string(),
                ignore: if ignore.is_none() {
                    vec![]
                } else {
                    ignore.unwrap().map(|x| x.to_string()).collect()
                },
                poll_interval: if poll_interval.is_none() { None } else { Some(poll_interval.unwrap().to_owned()) },
                write_delay: if write_delay.is_none() { None } else { Some(write_delay.unwrap().to_owned()) },
                name: name.to_owned(),
            });

            let updated_conf = toml::to_string(&conf).unwrap();

            fs::copy(&conf_file_path, format!("{}.bak", &conf_file_path)).unwrap();
            fs::write(&conf_file_path, updated_conf).unwrap();

            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            conn.get_mut()
                .write_all(&[0b0000_0010, 0xA])
                .expect("write failure");
        }
        //TODO: implement
        Some(("edit-config", args)) => {
            let scope = args.get_one::<String>("SCOPE").unwrap();
            let key = args.get_one::<String>("KEY");
            let value = args.get_many::<String>("VALUE");
            let no_reload = args.get_flag("no-reload");

            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            let mut buf = String::new();
            conn.get_mut().write_all(&[0b0000_0100, 0xA]).unwrap();
            conn.read_line(&mut buf).unwrap();
            buf.pop();

            let conf_file_path = buf;
            let mut conf_file =
                fs::File::open(&conf_file_path).expect("Failed to open config file");

            let mut buf = String::new();
            conf_file
                .read_to_string(&mut buf)
                .expect("Failed to open config file");
            let mut conf = config::CbakConfig::new(&buf);

            match scope.as_str() {
                "global" => {
                    if key.is_none() & value.is_none() {
                        println!("{:?}", conf.global);
                        return;
                    }

                    if value.is_none() {
                        match key.unwrap().as_str() {
                            "poll_interval" => {
                                println!("{:?}", conf.global.poll_interval);
                            },
                            "write_delay" => {
                                println!("{:?}", conf.global.write_delay);
                            },
                            "ignore" => {
                                println!("{:?}", conf.global.ignore);
                            },
                            _ => {
                                eprintln!("Invalid key");
                            }
                        }
                        return;
                    }

                    //let value = value.unwrap().collect::<Vec<&String>>();
                    match key.unwrap().as_str() {
                        "poll_interval" => {
                            let v = value.unwrap().collect::<Vec<&String>>();
                            if v.len() != 1 {
                                eprintln!("Invalid number of arguments");
                                return;
                            }
                            
                            if let Ok(n) = v[0].parse::<i32>() {
                                if n.is_negative() {
                                    eprintln!("Expected positive integer");
                                    return;
                                }
                                conf.global.poll_interval = n;
                            } else {
                                eprintln!("Expected positive integer");
                                return;
                            }
                        },
                        "write_delay" => {
                            let v = value.unwrap().collect::<Vec<&String>>();
                            if v.len() != 1 {
                                eprintln!("Invalid number of arguments");
                                return;
                            }
                            
                            if let Ok(n) = v[0].parse::<i32>() {
                                if n.is_negative() {
                                    eprintln!("Expected positive integer");
                                    return;
                                }
                                conf.global.write_delay = n;
                            } else {
                                eprintln!("Expected positive integer");
                                return;
                            }
                        },
                        "ignore" => {
                            conf.global.ignore = if value.is_none() { value.unwrap().collect::<Vec<&String>>().iter().map(|d| d.to_string()).collect::<Vec<String>>() } else { vec![] }
                        },
                        _ => {
                            eprintln!("Invalid key");
                            return;
                        }
                    }
                },
                name => {
                    let mut _watch = conf.watch.clone();
                    _watch.retain(|f| f.name == name);
                    let mut watch = _watch[0].clone();

                    if key.is_none() & value.is_none() {
                        println!("{:?}", watch);
                        return;
                    }

                    if value.is_none() {
                        match key.unwrap().as_str() {
                            "poll_interval" => {
                                watch.poll_interval = None;
                            },
                            "write_delay" => {
                                watch.write_delay = None;
                            },
                            "ignore" => {
                                watch.ignore = vec![];
                            },
                            _ => {
                                eprintln!("Invalid key");
                                return;
                            }
                        }
                    } else {
                        let value = value.unwrap().collect::<Vec<&String>>();
                        match key.unwrap().as_str() {
                            "poll_interval" => {
                                if value.len() != 1 {
                                    eprintln!("Invalid number of arguments");
                                    return;
                                }
                                
                                if let Ok(n) = value[0].parse::<i32>() {
                                    if n.is_negative() {
                                        eprintln!("Expected positive integer");
                                        return;
                                    }
                                    watch.poll_interval = Some(n);
                                } else {
                                    eprintln!("Expected positive integer");
                                    return;
                                }
                            },
                            "write_delay" => {
                                if value.len() != 1 {
                                    eprintln!("Invalid number of arguments");
                                    return;
                                }
                                
                                if let Ok(n) = value[0].parse::<i32>() {
                                    if n.is_negative() {
                                        eprintln!("Expected positive integer");
                                        return;
                                    }
                                    watch.write_delay = Some(n);
                                } else {
                                    eprintln!("Expected positive integer");
                                    return;
                                }
                            },
                            "ignore" => {
                                watch.ignore = value.iter().map(|d| d.to_string()).collect::<Vec<String>>();
                            },
                            _ => {
                                eprintln!("Invalid key");
                                return;
                            }
                        }
                    }
                    conf.watch.remove(conf.watch.iter().position(|f| f == &_watch[0]).unwrap());
                    conf.watch.push(watch);
                }
            }

            let updated_conf = toml::to_string(&conf).unwrap();

            fs::copy(&conf_file_path, format!("{}.bak", &conf_file_path)).unwrap();
            fs::write(&conf_file_path, updated_conf).unwrap();
            if !no_reload {
                let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
                let mut conn = BufReader::new(conn);
                conn.get_mut()
                    .write_all(&[0b0000_0010, 0xA])
                    .expect("write failure");
            }
        }
        Some(("unwatch", args)) => {
            let name = args.get_one::<String>("NAME").unwrap().to_owned(); 

            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            let mut buf = String::new();
            conn.get_mut().write_all(&[0b0000_0100, 0xA]).unwrap();
            conn.read_line(&mut buf).unwrap();
            buf.pop();

            let conf_file_path = buf;
            let mut conf_file =
                fs::File::open(&conf_file_path).expect("Failed to open config file");

            let mut buf = String::new();
            conf_file
                .read_to_string(&mut buf)
                .expect("Failed to open config file");
            let mut conf = config::CbakConfig::new(&buf);
            conf.watch.retain(|x| x.name != name);

            let updated_conf = toml::to_string(&conf).unwrap();

            fs::copy(&conf_file_path, format!("{}.bak", &conf_file_path)).unwrap();
            fs::write(&conf_file_path, updated_conf).unwrap();

            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            conn.get_mut()
                .write_all(&[0b0000_0010, 0xA])
                .expect("write failure");
        }
        Some(("list", _)) => {
            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            let mut buf = String::new();
            conn.get_mut().write_all(&[0b0000_0100, 0xA]).unwrap();
            conn.read_line(&mut buf).unwrap();
            buf.pop();

            let conf_file_path = buf;
            let mut conf_file =
                fs::File::open(&conf_file_path).expect("Failed to open config file");

            let mut buf = String::new();
            conf_file
                .read_to_string(&mut buf)
                .expect("Failed to open config file");
            let conf = config::CbakConfig::new(&buf);
            let table = conf
                .watch
                .iter()
                .map(|i| {
                    vec![
                        i.name.clone().cell(),
                        i.directory.clone().cell().justify(Justify::Right),
                    ]
                })
                .collect::<Vec<Vec<CellStruct>>>()
                .table()
                .title(vec![
                    "Name".cell().bold(true),
                    "Directory".cell().bold(true),
                ])
                .bold(true);
            print_stdout(table).unwrap();
        }
        _ => {
            eprintln!("Bad argument. (cbak help)?");
        }
    }
}
