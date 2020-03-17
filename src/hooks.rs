use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use std::ffi::OsString;

pub fn run_hooks(dir: &PathBuf, host_dir: &PathBuf, tag_dirs: &Vec<PathBuf>, simulate: bool) -> bool {

    let mut hooks_files = HashMap::new();

    // collect the hooks from the main dir
    match dir.read_dir() {
        Ok(dirs) => {
            for entry in dirs {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if entry.file_type().unwrap().is_file() {
                            hooks_files.insert(path.file_name().unwrap().to_os_string(), path);
                        }
                    }
                    Err(msg) => {
                        println!("{}", msg);
                    }
                }
            }
        }
        Err(msg) => {
            println!("{:?} {}", dir, msg);
        }
    }

    // collect the tag-specific hooks
    // hooks with the same file name will override those from the global directory
    for tag_dir in tag_dirs {
        match tag_dir.read_dir() {
            Ok(dirs) => {
                for entry in dirs {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if entry.file_type().unwrap().is_file() {
                                hooks_files.insert(path.file_name().unwrap().to_os_string(), path);
                            }
                        }
                        Err(msg) => {
                            println!("{}", msg);
                        }
                    }
                }
            }
            Err(msg) => {
                println!("{:?} {}", host_dir, msg);
            }
        }
    }

    // collect the host-specific hooks
    // hooks with the same file name will override those from the global directory
    // and tag directories
    match host_dir.read_dir() {
        Ok(dirs) => {
            for entry in dirs {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if entry.file_type().unwrap().is_file() {
                            hooks_files.insert(path.file_name().unwrap().to_os_string(), path);
                        }
                    }
                    Err(msg) => {
                        println!("{}", msg);
                    }
                }
            }
        }
        Err(msg) => {
            println!("{:?} {}", host_dir, msg);
        }
    }

    let mut keys = hooks_files.keys().collect::<Vec<&OsString>>();
    keys.sort();

    for file_name in keys {
        let path = hooks_files.get(file_name).unwrap();
        let s = path.as_os_str();


        if simulate {
            println!(":: Executing hook {:?}", path);
        } else {

            println!(":: Executing hook {:?}", file_name);
            let result = Command::new(s).status();
            match result {
                Ok(status) => {
                    if !status.success() {
                        match status.code() {
                            Some(code) => println!(":: Hook failed with status code: {}", code),
                            None => println!(":: Hook failed: terminated by signal"),
                        }
                        return false;
                    }
                }
                Err(msg) => {
                    println!(":: Failed to execute hook: {}", msg);
                    return false;
                }
            }
        }
    }

    return true;
}
