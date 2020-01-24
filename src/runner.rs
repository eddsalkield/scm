// TODO:
// rustfmt to format the code
// clippy to lint the code

use std::error::Error;

use std::io::{self, Read, Write};
use std::path::{PathBuf, Path};
use std::fs;
use std::collections::{HashMap, HashSet};
use std::env;
use std::os::unix::prelude::*;
use std::ffi::{OsStr, OsString};

use args::Args;
use hooks;
use file_ops::FS;
use toml::Value;
use regex::Regex;
use settings;

/// Prompts the user to answer yes or no to a prompt
/// Returns true on positive answer, false otherwise
///
/// A blank line, 'y', or 'yes', after that line has been trimmed and converted to lowercase, is
/// considered a positive answer. Anything else is considered negative.
fn ask(prompt: &str) -> bool {
    print!(">>> {} [Y/n] ", prompt);
    io::stdout().flush().unwrap();

    let mut input_text = String::new();
    io::stdin().read_line(&mut input_text).expect(
        "failed to read from stdin",
    );
    let answer = input_text.trim().to_lowercase();

    if answer == "yes" || answer == "y" || answer == "" {
        return true;
    }

    return false;
}

// TODO
fn substitute_variable_name(it: &mut impl Iterator<Item = u8>, r: &mut Vec<u8>) -> Result<(), io::Error> {
    let mut v = Vec::<u8>::new();
    // Fail if the next character read is not alphabetic, or an underscore

    if it.next() != Some(b'{') {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid path: unescaped '$' must be followed by a bracketed variable name, e.g. ${HOME}"));
    }

    let b = it.next().unwrap();
    if ! (b.is_ascii_alphabetic() || b == b'_') {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid path: variables must start with an alphabetic character or _"));
    }
    v.push(b);

    let mut complete = false;
    while let Some(b) = it.next() {
        if b == b'}' {
            complete = true;
            break;
        }
        else if ! (b.is_ascii_alphanumeric() || b == b'_') {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid path: variables may only contain alphanumeric characters of _"));
        }
        else {
            v.push(b);
        }
    }

    if ! complete {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid path: variables must end with a closing }"));
    }

    let var_name = OsStr::from_bytes(&v);
    match env::var_os(var_name) {
        Some(val) => {let mut result: Vec<u8> = val.as_bytes().to_vec(); r.append(&mut result)}
        None => println!("var_name is not defined in the environment"),
    }

    Ok(())
}

fn substitute_variables(string: &Path) -> Result<PathBuf, io::Error> {
    let mut r = Vec::<u8>::new();
    let pb = string.to_path_buf();
    let mut it = pb.to_str().unwrap().as_bytes().iter().cloned();
    while let Some(b) = it.next() {
        if b == b'\\' {
            let b2 = match it.next() {
                Some(v) => v,
                None => return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid path: ends in \\")),
            };
            r.push(b2);
        } else if b == b'$' {
            substitute_variable_name(&mut it, &mut r);
        } else {
            r.push(b);
        }
    }

    Ok(OsString::from_vec(r).into())
}

fn validate_settings_file(path: &Path) -> bool {
    path.is_file() && path.is_absolute()
}

fn parse_settings_file(path: &Path) -> Result<settings::Settings, io::Error> {
    let contents = fs::read_to_string(path)?;
    let value = toml::from_str(&contents).unwrap();
    Ok(value)
}

fn get_target(base_dir: Option<&Path>, settings: &settings::Settings) -> Result<PathBuf, io::Error> {
    if let Some(base_dir) = base_dir {
        if ! base_dir.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "base is not a valid base directory"));
        }
        if ! base_dir.is_absolute() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "base is not an absolute path"));
        }
    }

    let mut expanded_target: Option<PathBuf> = None;
    for t in settings.target.iter() {
        let cp = substitute_variables(&t.path)?;
        let candidate_path = cp.as_path();
        // Assert candidate_path is a subdir of base_dir
        
        if expanded_target.is_none() {
            if candidate_path.is_dir() && candidate_path.is_absolute() {
                expanded_target = Some(candidate_path.to_path_buf());
            }
        }
    }

    match expanded_target {
        Some(t) => Ok(t),
        None => Err(io::Error::new(io::ErrorKind::InvalidInput, "no valid target found")),
    }
}

pub struct Runner<'a> {
    args: &'a Args,
}

impl<'a> Runner<'a> {
    pub fn new(args: &Args) -> Runner {
        Runner { args: args }
    }

    pub fn install(&self) -> bool {

        let args = self.args;

        let f: FS = FS::new(self.args.force);

        let mut base_directory = args.dir.clone();
        let mut base_settings_file = base_directory.clone();
        base_settings_file.push("settings.toml");

        let mut global_target_dir = args.target_dir.clone();
        if validate_settings_file(&base_settings_file) {
            let mut settings = parse_settings_file(&base_settings_file).unwrap();
            global_target_dir = get_target(None, &settings).unwrap();
        }

        for package1 in &args.packages {
            println!(":: Installing package {:?}", package1);

            let mut package_base = args.dir.clone();
            package_base.push(package1);

            let mut global_settings_file = package_base.clone();
            global_settings_file.push("settings.toml");

            let mut target_dir = global_target_dir.clone();
            if validate_settings_file(&global_settings_file) {
                let mut settings = parse_settings_file(&global_settings_file).unwrap();
                target_dir = get_target(None, &settings).unwrap();
            }

            println!(":: Will install from {:?}", package_base);
            println!("                  to {:?}", target_dir);

            // only prompt if not in test mode and haven't added the 'no confirm' flag
            if !args.no_confirm && !args.test {
                if !ask("Continue?") {
                    println!(":: Aborting installation of {:?}", package1);
                    continue;
                }
            }

            let mut global_hooks_base = package_base.clone();
            global_hooks_base.push("hooks");

            // run the pre-up hooks

            let mut pre_up_hooks_dir = global_hooks_base.clone();
            pre_up_hooks_dir.push("pre-up");
            let mut host_pre_up_hooks_dir = package_base.clone();
            host_pre_up_hooks_dir.push("hosts");
            host_pre_up_hooks_dir.push(&args.hostname);
            host_pre_up_hooks_dir.push("hooks");
            host_pre_up_hooks_dir.push("pre-up");

            println!(":: Executing pre-up hooks.");
            let ok = hooks::run_hooks(&pre_up_hooks_dir, &host_pre_up_hooks_dir, args.test);
            if !ok {
                return false;
            }


            let mut global_files_base = package_base.clone();
            global_files_base.push("files");

            println!(":: Creating parent dirs where required.");
            // create all the directories required
            let dirs = f.get_dirs_to_create(&global_files_base);
            for dir in dirs {
                let base = dir.strip_prefix(&global_files_base).unwrap();
                let new_dir = target_dir.join(base);

                if !args.test {
                    let result = f.create_dir_all(&new_dir);
                    match result {
                        Ok(_) => (),
                        Err(msg) => println!(":: Creating {:?} failed: {}", new_dir, msg),
                    }
                }

            }

            // host specific config
            let mut host_files_base = package_base.clone();
            host_files_base.push("hosts");
            host_files_base.push(&args.hostname);
            host_files_base.push("files");

            let mut host_files: Vec<PathBuf> = vec![];

            if f.dir_exists(&host_files_base) {

                let host_dirs = f.get_dirs_to_create(&host_files_base);
                for dir in host_dirs {
                    let base = dir.strip_prefix(&host_files_base).unwrap();
                    let new_dir = target_dir.join(base);

                    if !args.test {
                        let result = f.create_dir_all(&new_dir);
                        match result {
                            Ok(_) => (),
                            Err(msg) => {
                                println!(":: Creating {:?} failed!\n{}", new_dir, msg);
                                return false;
                            }
                        }
                    }
                }

                // symlink the files
                host_files = f.get_files_to_symlink(&host_files_base);
            }

            let files = f.get_files_to_symlink(&global_files_base);

            // map destinations to link targets
            // this method allows host-specfic files to take precedence
            let mut dests: HashMap<PathBuf, PathBuf> = HashMap::new();
            for file in host_files {
                let dest = target_dir.join(
                    file.strip_prefix(&host_files_base).unwrap(),
                );
                dests.insert(dest, file.clone());
            }

            for file in files {
                let dest = target_dir.join(
                    file.strip_prefix(&global_files_base)
                        .unwrap(),
                );
                if !dests.contains_key(&dest) {
                    dests.insert(dest, file.clone());
                }
            }

            println!(":: Creating links.");
            let mut was_failure = false;
            for (dest, file) in dests {
                // dest is the new file to be created
                // it should be a symbolic link pointing to file
                let ok = f.create_link(&dest, &file, args.test);
                if !ok {
                    was_failure = true;
                }
            }

            if was_failure {
                println!(
                    ":: One or more files failed to link, exiting without running post-up hooks."
                );
                return false;
            }


            // Now for the post-up hooks!

            println!(":: Executing post-up hooks.");

            let mut post_up_hooks_dir = global_hooks_base.clone();
            post_up_hooks_dir.push("post-up");
            let mut host_post_up_hooks_dir = package_base.clone();
            host_post_up_hooks_dir.push("hosts");
            host_post_up_hooks_dir.push(&args.hostname);
            host_post_up_hooks_dir.push("hooks");
            host_post_up_hooks_dir.push("post-up");

            let ok = hooks::run_hooks(&post_up_hooks_dir, &host_post_up_hooks_dir, args.test);
            if !ok {
                return false;
            }

        }

        return true;
    }

    pub fn uninstall(&self) -> bool {

        let args = self.args;

        let f: FS = FS::new(self.args.force);

        let mut base_directory = args.dir.clone();
        let mut base_settings_file = base_directory.clone();
        base_settings_file.push("settings.toml");

        let mut global_target_dir = args.target_dir.clone();
        if validate_settings_file(&base_settings_file) {
            let mut settings = parse_settings_file(&base_settings_file).unwrap();
            global_target_dir = get_target(None, &settings).unwrap();
        }

        for package1 in &args.packages {
            println!(":: Removing package {:?}", package1);

            let mut package_base = args.dir.clone();
            package_base.push(package1);

            let mut global_settings_file = package_base.clone();
            global_settings_file.push("settings.toml");

            let mut target_dir = global_target_dir.clone();
            if validate_settings_file(&global_settings_file) {
                let mut settings = parse_settings_file(&global_settings_file).unwrap();
                target_dir = get_target(None, &settings).unwrap();
            }

            println!(":: Will remove all links in {:?}", target_dir);
            println!("     that point to files in {:?}", package_base);

            // only prompt if not in test mode and haven't added the 'no confirm' flag
            if !args.no_confirm && !args.test {
                if !ask("Continue?") {
                    println!(":: Aborting removal of {:?}", package1);
                    continue;
                }
            }

            let mut global_hooks_base = package_base.clone();
            global_hooks_base.push("hooks");


            // run the pre-down hooks

            println!(":: Executing pre-down hooks.");
            let mut pre_down_hooks_dir = global_hooks_base.clone();
            pre_down_hooks_dir.push("pre-down");
            let mut host_pre_down_hooks_dir = package_base.clone();
            host_pre_down_hooks_dir.push(format!("hosts/{}/hooks/pre-down/", &args.hostname));
            println!("{:?}", host_pre_down_hooks_dir);

            let ok = hooks::run_hooks(&pre_down_hooks_dir, &host_pre_down_hooks_dir, args.test);
            if !ok {
                return false;
            }


            let mut global_files_base = package_base.clone();
            global_files_base.push("files");

            // host specific config
            let mut host_files_base = package_base.clone();
            host_files_base.push(format!("hosts/{}/files/", &args.hostname));

            let mut host_files: Vec<PathBuf> = vec![];

            if f.dir_exists(&host_files_base) {

                let host_dirs = f.get_dirs_to_create(&host_files_base);
                for dir in host_dirs {
                    let base = dir.strip_prefix(&host_files_base).unwrap();
                    let new_dir = target_dir.join(base);

                    let result = f.create_dir_all(&new_dir);
                    match result {
                        Ok(_) => println!("created ok!"),
                        Err(msg) => println!("fail: {}", msg),
                    }

                }

                // symlink the files
                host_files = f.get_files_to_symlink(&host_files_base);
            }

            let files = f.get_files_to_symlink(&global_files_base);

            // map destinations to link targets
            // this method allows host-specfic files to take precedence
            let mut dests: HashSet<PathBuf> = HashSet::new();
            for file in host_files {
                let dest = target_dir.join(
                    file.strip_prefix(&host_files_base).unwrap(),
                );
                dests.insert(dest);
            }

            for file in files {
                let dest = target_dir.join(
                    file.strip_prefix(&global_files_base)
                        .unwrap(),
                );
                if !dests.contains(&dest) {
                    dests.insert(dest);
                }
            }

            for dest in dests {

                // if the file doesn't exist, then don't do anything
                if !f.exists(&dest) {
                    continue;
                }

                // check if we should remove it
                // resolve the symlinks and check where it points, and whether force is set
                match dest.canonicalize() {
                    Ok(path) => {
                        if !path.starts_with(&package_base) {
                            if !args.force {
                                println!(
                                    ":: Existing file does not point to package base, not removing.\n   --> {:?}",
                                    &dest
                                );
                                continue;
                            }
                        }
                    }
                    Err(msg) => {
                        println!(":: Error checking existing file {:?} : {}", &dest, msg);
                    }
                }

                // delete!
                println!(":: Removing {:?}", &dest);
                if !args.test {
                    let res;
                    if dest.is_dir() {
                        res = f.remove_dir_all(&dest);
                    } else {
                        res = f.remove_file(&dest);
                    }
                    match res {
                        Ok(_) => (),
                        Err(msg) => {
                            println!("Failed to remove {:?} : {}", &dest, msg);
                            return false;
                        }
                    }
                }

            }


            // Now for the post-down hooks!

            println!(":: Executing post-down hooks.");
            let mut post_down_hooks_dir = global_hooks_base.clone();
            post_down_hooks_dir.push("post-down");
            let mut host_post_down_hooks_dir = package_base.clone();
            host_post_down_hooks_dir.push("hosts");
            host_post_down_hooks_dir.push(&args.hostname);
            host_post_down_hooks_dir.push("hooks");
            host_post_down_hooks_dir.push("post-down");

            let ok = hooks::run_hooks(&post_down_hooks_dir, &host_post_down_hooks_dir, args.test);
            if !ok {
                return false;
            }


        }

        return true;
    }

    pub fn add(&self) -> bool {
        // get the subcommand arguments - guaranteed to be present because this function only
        // called when add subcommand used
        let add_args = match &self.args.add_args {
            &Some(ref args) => args,
            _ => panic!("should never happen"),
        };

        let args = self.args;

        let f: FS = FS::new(self.args.force);

        println!(":: Adding {:?}", add_args.filename);
        println!(":: --> package {:?}", add_args.package);
        println!(
            ":: Host-specific mode is {}.",
            if add_args.host_specific { "on" } else { "off" }
        );

        let mut target = self.args.dir.clone();
        target.push(&add_args.package);

        if add_args.host_specific {
            target.push("hosts");
            target.push(&self.args.hostname);
        }
        target.push("files");

        let file_base = match add_args.filename.strip_prefix(&self.args.target_dir) {   // TODO fix
            Ok(path) => path,
            Err(_) => {
                println!("ERR: File to add must be in the target directory.");
                return false;
            }
        };
        target.push(file_base);

        println!(":: File will be moved to {:?}.", &target);
        println!(":: And link created in original location.");

        // only prompt if not in test mode and haven't added the 'no confirm' flag
        if !args.no_confirm && !args.test {
            if !ask("Continue?") {
                println!(":: Aborting add operation.");
                return true;
            }
        }


        let exists = f.exists(&target);
        if exists {
            if !self.args.force {
                println!(":: Target file exists in repo, not overwriting.");
                return false;
            } else {
                println!(":: Overwriting existing file in repo.");
                if !self.args.test {
                    let res;
                    if target.is_dir() {
                        res = f.remove_dir_all(&target);
                    } else {
                        res = f.remove_file(&target);
                    }
                    match res {
                        Ok(_) => {
                            println!("Deleted {:?}", &target);
                        }
                        Err(msg) => {
                            println!("Failed to remove {:?} : {}", &target, msg);
                            return false;
                        }
                    }
                }
            }
        }

        if !self.args.test {
            let res = f.create_dir_all(&target.parent().unwrap().to_owned());
            match res {
                Ok(_) => (),
                Err(msg) => {
                    println!(
                        "ERR: Failed creating target directory {:?}\n{}",
                        &target.parent().unwrap().to_owned(),
                        msg
                    );
                    return false;
                }
            }

            match f.rename(&add_args.filename, &target) {
                Ok(_) => (),
                Err(msg) => {
                    println!("Moving file to repo failed: {}", msg);
                    return false;
                }
            }
        }

        let success = f.create_link(&add_args.filename, &target, self.args.test);
        if success {
            return true;
        } else {
            return false;
        }
    }
}
