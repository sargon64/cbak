use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    vec,
};

use clap::{Parser, Subcommand};
use cli_table::{format::Justify, print_stdout, Cell, CellStruct, Style, Table};
use interprocess::local_socket::{LocalSocketStream, NameTypeSupport};

mod config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CbakArgs {
    #[command(subcommand)]
    action: CbakAction,
}

#[derive(Subcommand, PartialEq)]
enum CbakAction {
    /// Adds a directory to the watchlist
    Watch {
        directory: String,
        name: String,
        poll_interval: Option<i32>,
        write_delay: Option<i32>,
        ignore: Option<Vec<String>>,
    },
    /// Removes a directory from the watchlist
    Unwatch {
        name: String,
    },
    List,
    /// Edit configuration
    Config,
    ///sends ack
    Ack,
}

fn main() {
    let args = CbakArgs::parse();

    let sock_name = {
        use NameTypeSupport::*;
        match NameTypeSupport::query() {
            OnlyPaths => "/tmp/cbak.sock",
            OnlyNamespaced | Both => "@cbak.sock",
        }
    };
    match args.action {
        CbakAction::Ack => {
            let conn = LocalSocketStream::connect(sock_name).expect("failed to connect to socket");
            let mut conn = BufReader::new(conn);
            conn.get_mut()
                .write_all(&[0b0000_0001, 0xA])
                .expect("write failure");
       }
        CbakAction::Watch {
            directory,
            name,
            ignore,
            poll_interval,
            write_delay,
        } => {
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
                    ignore.unwrap()
                },
                poll_interval,
                write_delay,
                name,
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
        CbakAction::Config => {
            println!("Not implemented yet");
        }
        CbakAction::Unwatch { name } => {
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
        CbakAction::List => {
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
    }
}
