/// 

use core::time;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime, io::{self, BufReader, Read, Write, BufRead}, thread::JoinHandle,
};

use interprocess::local_socket::{NameTypeSupport, LocalSocketListener, LocalSocketStream};
use rayon::prelude::*;
use regex::RegexSet;
use bitflags::bitflags;

mod config;

#[derive(Debug)]
enum FileChanges<'a> {
    File(Vec<&'a PathBuf>),
    Modify(Vec<&'a PathBuf>),
}

/// Holds all the contents of a directory
/// contents is the pooled contents of all of the subdirs & the root directory (excluding dirs)
#[derive(Debug)]
struct DirContents {
    root: PathBuf,
    subdirs: Vec<PathBuf>,
    contents: Vec<PathBuf>,
}

fn handle_socket_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    match conn {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("Incoming connection failed: {}", e);
            None
        }
    }
}


fn main() {
    let config = config::CbakConfig::new().unwrap();

    let mut handles = vec![];
    // for every [[watch]] block in the config, spawn a thread to watch that dir.
    for i in config.watch {
        if !Path::new(&i.directory).join(".git/").exists() {
            Command::new("git")
                .arg("init")
                .current_dir(&i.directory)
                .output()
                .unwrap();
        }
        handles.push(std::thread::spawn(move || run(i)));
    }
    
    //sleep the current thread
    loop {
        //Listen for cli updates on main thread
        let name = {
            use NameTypeSupport::*;
            match NameTypeSupport::query() {
                OnlyPaths => "/tmp/cbak.sock",
                OnlyNamespaced | Both => "@cbak.sock"
            }
        };

        let listener = match LocalSocketListener::bind(name) {
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                eprintln!(
                    "Error: Could not bind to socket {}. Please check if socket is in use.", name
                );
                return;
            }
            x => x.expect("Socket error")
        };

        println!("Bound to socket {}", name);

        //The control sceme will be as follows
        //0000_0000 = no flags
        //0000_0001 = ack
        //0000_0010 = config update
        //0000_0100 = request config path
        // -- further bits reserved
        let mut buf = String::new();
        for conn in listener.incoming().filter_map(handle_socket_error) {
            buf.clear();
            let mut conn = BufReader::new(conn);

            conn.read_line(&mut buf).expect("read failure");
            let b1 = buf.trim().as_bytes().first().unwrap_or(&0);
            if b1 & 0b0000_0001 == 0b0000_0001 {
                println!("{}", "w");
            }
             if b1 & 0b0000_0010 == 0b0000_0010 {
                println!("{}", "w");
            }
             if b1 & 0b0000_0100 == 0b0000_0100 {
                //println!("{}", "w");
                conn.get_mut().write_all(format!("{}\n", config.config_file_path.to_str().unwrap()).as_bytes()).unwrap();
            }
            //conn.get_mut().write_all(b"hello\n").expect("write failure");
            buf.clear();
        }
    }
}

fn run(config: config::DirConfig) -> ! {
    // main watch loop
    loop {
        let files = get_all_files_filtered(Path::new(&config.directory), &config.ignore).unwrap();

        let res = wait_until_changed(&files, config.poll_interval, config.write_delay);

        Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&config.directory)
            .output()
            .unwrap();

        Command::new("git")
            .arg("rm")
            .arg("-f")
            .arg("--cached")
            .args(
                get_all_files_nfiltered(Path::new(&config.directory), &config.ignore)
                    .unwrap()
                    .contents
                    .iter()
                    .map(|i| {
                        i.strip_prefix(&config.directory)
                            .unwrap()
                            .to_str()
                            .unwrap_or("")
                    })
                    .collect::<Vec<&str>>(),
            )
            .current_dir(&config.directory)
            .output()
            .unwrap();
        Command::new("git")
            .arg("rm")
            .arg("-f")
            .arg("--cached")
            .args(
                get_all_files_nfiltered(Path::new(&config.directory), &config.ignore)
                    .unwrap()
                    .subdirs
                    .iter()
                    .map(|i| {
                        i.strip_prefix(&config.directory)
                            .unwrap()
                            .to_str()
                            .unwrap_or("")
                    })
                    .collect::<Vec<&str>>(),
            )
            .current_dir(&config.directory)
            .output()
            .unwrap();

        Command::new("git")
            .arg("commit")
            .args(["-m", "auto commit"])
            .current_dir(&config.directory)
            .output()
            .unwrap();

        println!("{:?}", res.unwrap_or(FileChanges::File(vec![])));
    }
}

///Waits until any files in a DirContents is changed 
fn wait_until_changed(
    dir: &DirContents,
    poll_time: i32,
    wait_time: i32,
) -> Result<FileChanges, Box<dyn std::error::Error>> {
    let cache_root_time = (&dir.root, dir.root.metadata()?.modified()?);
    let cache_subdir_time = dir
        .subdirs
        .par_iter()
        .map(|i| (i, i.metadata().unwrap().modified().unwrap()))
        .collect::<Vec<(&PathBuf, SystemTime)>>();
    let cache_contents_time = dir
        .contents
        .par_iter()
        .map(|i| (i, i.metadata().unwrap().modified().unwrap()))
        .collect::<Vec<(&PathBuf, SystemTime)>>();
    loop {
        std::thread::sleep(time::Duration::from_secs(poll_time as u64));
        let root_time = (&dir.root, dir.root.metadata()?.modified()?);
        let mut subdir_time = dir
            .subdirs
            .par_iter()
            .map(|i| {
                (
                    i,
                    i.metadata()
                        .unwrap_or_else(|_| unsafe { std::mem::zeroed() })
                        .modified()
                        .unwrap_or_else(|_| unsafe { std::mem::zeroed() }),
                )
            })
            .collect::<Vec<(&PathBuf, SystemTime)>>();
        let mut contents_time = dir
            .contents
            .par_iter()
            .map(|i| {
                (
                    i,
                    i.metadata()
                        .unwrap_or_else(|_| unsafe { std::mem::zeroed() })
                        .modified()
                        .unwrap_or_else(|_| unsafe { std::mem::zeroed() }),
                )
            })
            .collect::<Vec<(&PathBuf, SystemTime)>>();
        if cache_root_time != root_time {
            return Ok(FileChanges::File(vec![root_time.0]));
        }
        if cache_subdir_time != subdir_time {
            subdir_time.retain(|i| cache_subdir_time.binary_search(i).is_err());
            return Ok(FileChanges::File(
                subdir_time.par_iter().map(|i| i.0).collect(),
            ));
        }
        if cache_contents_time != contents_time {
            loop {
                std::thread::sleep(time::Duration::from_secs(wait_time as u64));
                if contents_time
                    == dir
                        .contents
                        .par_iter()
                        .map(|i| {
                            (
                                i,
                                i.metadata()
                                    .unwrap_or_else(|_| unsafe { std::mem::zeroed() })
                                    .modified()
                                    .unwrap_or_else(|_| unsafe { std::mem::zeroed() }),
                            )
                        })
                        .collect::<Vec<(&PathBuf, SystemTime)>>()
                {
                    break;
                } else {
                    contents_time = dir
                        .contents
                        .par_iter()
                        .map(|i| {
                            (
                                i,
                                i.metadata()
                                    .unwrap_or_else(|_| unsafe { std::mem::zeroed() })
                                    .modified()
                                    .unwrap_or_else(|_| unsafe { std::mem::zeroed() }),
                            )
                        })
                        .collect::<Vec<(&PathBuf, SystemTime)>>();
                }
            }
            //println!("{:?}", contents_time);
            contents_time.retain(|i| cache_contents_time.binary_search(i).is_err());
            return Ok(FileChanges::Modify(
                contents_time.par_iter().map(|i| i.0).collect(),
            ));
        }
    }
}

/// Gets all the files in a directory, within a DirContents struct, filtered by the ignore param
fn get_all_files_filtered(dir: &Path, ignore: &RegexSet) -> std::io::Result<DirContents> {
    let r = dir.read_dir()?;
    let mut paths = Vec::new();
    let mut subdirs = Vec::new();
    for file in r {
        match file {
            Ok(f) => {
                match f.path().is_dir() {
                    true => {
                        let mut r = get_all_files_filtered(&f.path(), ignore)?;

                        // Add subdirs to list
                        subdirs.push(r.root);
                        subdirs.append(&mut r.subdirs);

                        paths.append(&mut r.contents);
                    }
                    false => paths.push(f.path()),
                }
            }
            Err(_) => continue,
        }
    }
    // Remvoe duplicate subdirs
    subdirs.sort();
    subdirs.dedup();

    Ok(DirContents {
        root: dir.to_path_buf(),
        subdirs: subdirs
            .par_iter()
            .filter(|p| !ignore.is_match(p.to_str().unwrap()))
            .map(|p| p.to_owned())
            .collect::<Vec<PathBuf>>(),
        contents: paths
            .par_iter()
            .filter(|p| !ignore.is_match(p.to_str().unwrap()))
            .map(|p| p.to_owned())
            .collect::<Vec<PathBuf>>(),
    })
}

/// Gets all the files in a directory, within a DirContents struct, that would of been removed by the ignore param
fn get_all_files_nfiltered(dir: &Path, ignore: &RegexSet) -> std::io::Result<DirContents> {
    let r = dir.read_dir()?;
    let mut paths = Vec::new();
    let mut subdirs = Vec::new();
    for file in r {
        match file {
            Ok(f) => {
                match f.path().is_dir() {
                    true => {
                        let mut r = get_all_files_filtered(&f.path(), ignore)?;
                        // Add subdirs to list
                        subdirs.push(r.root);
                        subdirs.append(&mut r.subdirs);

                        paths.append(&mut r.contents);
                    }
                    false => paths.push(f.path()),
                }
            }
            Err(_) => continue,
        }
    }
    // Remvoe duplicate subdirs
    subdirs.sort();
    subdirs.dedup();

    Ok(DirContents {
        root: dir.to_path_buf(),
        subdirs: subdirs
            .par_iter()
            .filter(|p| ignore.is_match(p.to_str().unwrap()))
            .map(|p| p.to_owned())
            .collect::<Vec<PathBuf>>(),
        contents: paths
            .par_iter()
            .filter(|p| ignore.is_match(p.to_str().unwrap()))
            .map(|p| p.to_owned())
            .collect::<Vec<PathBuf>>(),
    })
}
