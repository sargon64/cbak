use core::time;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use rayon::prelude::*;
use regex::RegexSet;

mod config;

#[derive(Debug)]
enum FileChanges<'a> {
    File(Vec<&'a PathBuf>),
    Modify(Vec<&'a PathBuf>),
}

#[derive(Debug)]
struct DirContents {
    root: PathBuf,
    subdirs: Vec<PathBuf>,
    contents: Vec<PathBuf>,
}

fn main() {
    let config = config::CbakConfig::new().unwrap();
    //println!("{:?}", config);
    // let config = Arc::new(config::CbakConfig::new(
    //     std::env::current_dir()
    //         .unwrap()
    //         .join("config_test\\")
    //         .to_str()
    //         .unwrap(),
    //     RegexSet::new(&[r"\\.git"]).unwrap(),
    // ));
    for i in config.watch {
        // let repo = Some(match Repository::open(&i.directory) {
        //     Ok(r) => r,
        //     Err(_) => {
        //         Repository::init(&i.directory).unwrap()
        //     },
        // });
        if !Path::new(&i.directory).join(".git/").exists() {
            Command::new("git")
                .arg("init")
                .current_dir(&i.directory)
                .output()
                .unwrap();
        }
        std::thread::spawn(move || run(i));
    }
    loop {
        std::thread::sleep(time::Duration::from_micros(1))
    }
}

fn run(config: config::DirConfig) -> ! {
    //let mut res_cache: FileChanges;
    loop {
        let files = get_all_files_filtered(Path::new(&config.directory), &config.ignore).unwrap();
        let res = wait_until_changed(&files);

        //    if repo.is_some() {
        //     repo.unwrap().commit(Some("HEAD"), &Signature::now("cbak", "cbak@cbak.cum"), &Signature::now("cbak", "cbak@cbak.cum"), "Automatic commit", "");
        //    }
        // println!(
        //     "{:?}",
        //     get_all_files_nfiltered(Path::new(&config.directory), &config.ignore)
        //         .unwrap()
        //         .contents
        //         .iter()
        //         .map(|i| i
        //             .strip_prefix(&config.directory)
        //             .unwrap()
        //             .to_str()
        //             .unwrap_or(""))
        //         .collect::<Vec<&str>>()
        // );
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

        // println!(
        //     "{:?}",
        //     get_all_files_nfiltered(Path::new(&config.directory), &config.ignore).unwrap()
        // );
        println!("{:?}", res.unwrap_or(FileChanges::File(vec![])));
    }
}

// #[must_use]
// fn wait_until_changed(path: &Path) -> Result<(FileChanges), Box<dyn std::error::Error>> {
//     let cache = path.metadata()?.modified()?;
//     loop {
//         println!("{:?} : {:?}", cache, path.metadata()?.modified()?);
//         std::thread::sleep(time::Duration::from_secs(5));
//         if path.metadata()?.modified()? != cache {break};
//     }
//     Ok((FileChanges::Modify))
// }

// fn ignore_files(dir: &DirContents) {}

fn wait_until_changed(dir: &DirContents) -> Result<FileChanges, Box<dyn std::error::Error>> {
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
        std::thread::sleep(time::Duration::from_secs(5));
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
                std::thread::sleep(time::Duration::from_secs(5));
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
