use std::{
    fs,
    io::{BufRead, BufReader, Read, Write}, vec,
};

use clap::{Parser, Subcommand};
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
    // Adds a directory to the watchlist
    Watch {
        directory: String,
        name: String,
        poll_interval: Option<i32>,
        write_delay: Option<i32>,
        ignore: Option<Vec<String>>,
    },
    // Removes a directory from the watchlist
    UnWatch,
    // Edit configuration
    Config,
    //sends ack
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
            let mut buf = String::new();
            conn.get_mut()
                .write_all(&[0b0000_0001, 0xA])
                .expect("write failure");
            conn.read_line(&mut buf).expect("read failure");
            //println!("{}", buf.trim());
            //let ack: [u8; 1] = [0b0000_0001];
            //conn.write(&ack).expect("write failed");
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
            let mut conf_file = fs::File::open(&conf_file_path).expect("Failed to open config file");

            let mut buf = String::new();
            conf_file
                .read_to_string(&mut buf)
                .expect("Failed to open config file");
            //println!("{:?}", &buf);
            let mut conf: config::_CbakConfig = toml::from_str(&buf).unwrap();


    
            conf.watch.push(config::_DirConfig {
                directory: fs::canonicalize(directory).expect("Not a valid directory").to_str().unwrap().to_string(),
                ignore: if ignore.is_none() {vec![]} else {ignore.unwrap()},
                poll_interval,
                write_delay,
                name,
            });
            let updated_conf = toml::to_string(&conf).unwrap();

            fs::copy(&conf_file_path, format!("{}.bak", &conf_file_path)).unwrap();
            fs::write(&conf_file_path, updated_conf).unwrap();
        }
        // CbakAction::Config => {
        //     let mut conn = LocalSocketStream::connect(name).expect("failed to connect to socket");
        //     let mut conn = BufReader::new(conn);
        //     let get_config_path: [u8; 1] = [0b0000_0100];
        //     conn.get_mut().write_all(&get_config_path).expect("write failed");
        //     let mut buf = String::new();
        //     //conn.read_line(&mut buf).unwrap();
        // }
        _ => {}
    }
}
