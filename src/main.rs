use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{TryRecvError, channel};
//use ctrlc;
//use std::thread;
//use walkdir::WalkDir;
//use path_absolutize::Absolutize;
use std::env;
use std::path::Path;
use std::fs::OpenOptions;
use argparse::{ArgumentParser, Store};
//use structopt::StructOpt;
/*
/// Tests PKGBUILDS
#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "abuildtester")]
struct ABuildTesterConfig {
    /// Output file.  Defaults to ./results.txt
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}
*/
fn main() {
    /*
    let opts = ABuildTesterConfig::from_args();
    //let mut orig_dir = env::current_dir().unwrap();
    //let mut output_path;
    let opt_output = opts.clone().output;
    let output_path = match opt_output {
        Some(x) => x,
        None => {
            let mut y = env::current_dir().unwrap();
            y.push("results.txt");
            y
        }
    };
    */
    let mut output_path = env::current_dir().unwrap();
    output_path.push("results.txt");
    
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Tests APKBUILDs");
        ap.refer(&mut output_path)
            .add_option(&["-o", "--output"], Store, "Path to results file.  Default is ./results.txt");
        ap.parse_args_or_exit();    
    }
    
    let mut completed_packages: Vec<String> = Vec::new();
    if output_path.exists() {
        for (num, line) in std::fs::read_to_string(output_path.clone()).unwrap().replace("\r", "").split("\n").enumerate() {
            if line.trim() != "" {
                let parts = line.split_once(": ").expect(format!("Error parsing at line {}", num + 1).as_str());
                completed_packages.push(parts.0.to_string())
            }
        }
    }
    let dir_entries = std::fs::read_dir("./").expect("Error reading current directory");
    let directories = dir_entries.filter(|x| x.as_ref().unwrap().path().is_dir());
    //println!("{:?}", directories.map(|x| x.as_ref().unwrap().path().display().to_string()).collect::<Vec<String>>())
    for directory in directories {
        let path = directory.unwrap().path();
        if !completed_packages.contains(&path.file_name().unwrap().to_str().unwrap().to_string()) {
            env::set_current_dir(path.clone()).expect(format!("Unable to change directory to {}", path.clone().display()).as_str());
            let (tx, rx) = channel();
            let tx_clone = tx.clone();
            ctrlc::set_handler(move || tx_clone.send("ctrlc").expect("Could not send signal on channel."))
                .expect("Error setting Ctrl-C handler");
            let (tx2, rx2) = channel();
            let mut works = false;
            if Path::new("./APKBUILD").exists() {
                
                let mut child = Command::new("abuild").arg("-R").stdout(Stdio::inherit()).stderr(Stdio::inherit()).stdin(Stdio::inherit()).spawn().expect(format!("Error spawning abuild -R for package '{}'", path.file_name().unwrap().to_str().unwrap()).as_str());
                
                let child_id = child.id() as i32;
                std::thread::spawn(move || {
                    let mut status = match rx.try_recv() {
                        Ok(rx) => rx,
                        Err(TryRecvError::Empty) => "empty",
                        Err(TryRecvError::Disconnected) => "disconnected", 
                    };
                    while status != "ctrlc" && status != "done" {
                        status = match rx.try_recv() {
                            Ok(rx) => rx,
                            Err(TryRecvError::Empty) => "empty",
                            Err(TryRecvError::Disconnected) => "disconnected",
                        };
                    }
                    if status == "ctrlc" {
                        tx2.send("quit").unwrap();
                        nix::sys::signal::kill(
                            nix::unistd::Pid::from_raw(child_id), 
                            nix::sys::signal::Signal::SIGINT
                        ).expect("cannot send ctrl-c");
                    } else {
                        tx2.send("continue").unwrap();
                    }
                });
                
                works = child.wait().expect("abuild not running").success();
                tx.send("done").unwrap();
                let status = rx2.recv().unwrap();
                if status == "quit" {
                    return;
                }
                
            }
            let out_string: String;
            if works {
                out_string = "PASS".to_string();
            } else {
                out_string = "FAIL".to_string();
            }
            {
                let mut out_file = OpenOptions::new().append(true).create(true).open(output_path.clone()).unwrap();
                out_file.write_all(format!("{}: {}\n", path.file_name().unwrap().to_str().unwrap(), out_string).as_bytes()).expect("Unable to write to results file.");
            }
            env::set_current_dir("../").unwrap();
        }
    }
}
