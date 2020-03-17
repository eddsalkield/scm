use std::io::{self, Write};
use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

use args::Args;
use hooks;
use file_ops::FS;


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

        for package1 in &args.packages {
            println!(":: Installing package {:?}", package1);

            let mut package_base = args.dir.clone();
            package_base.push(package1);

            println!(":: Will install from {:?}", package_base);
            println!("                  to {:?}", args.target_dir);

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
            host_pre_up_hooks_dir.push(format!("hosts/{}/hooks/pre-up/",
                &args.hostname));

            let mut tag_pre_up_hooks_dirs = Vec::new();
            for tag in &args.tags {
                let mut pre_up_tag_dir = package_base.clone();
                pre_up_tag_dir.push(format!("tags/{}/hooks/pre-up/", &tag));
                tag_pre_up_hooks_dirs.push(pre_up_tag_dir);
            }

            println!(":: Executing pre-up hooks.");
            let ok = hooks::run_hooks(&pre_up_hooks_dir, &host_pre_up_hooks_dir, &tag_pre_up_hooks_dirs, args.test);
            if !ok {
                return false;
            }

            println!(":: Creating parent dirs where required.");

            let mut global_files_base = package_base.clone();
            global_files_base.push("files");

            // create all the directories required
            f.create_dirs(&global_files_base, &args.target_dir, args.test);
            let files = f.get_files_to_symlink(&global_files_base);

            // host specific config
            let mut host_files_base = package_base.clone();
            host_files_base.push(format!("hosts/{}/files/", &args.hostname));

            let mut host_files: Vec<PathBuf> = vec![];
            if f.dir_exists(&host_files_base) {
                if (!f.create_dirs(&host_files_base, &args.target_dir, args.test)) {
                    return false;
                }

                // symlink the files
                host_files = f.get_files_to_symlink(&host_files_base);
            }

            // tag specific config
            let mut tag_files: Vec<(PathBuf, Vec<PathBuf>)> = vec![];
            for tag in &args.tags {
                let mut tag_files_base = package_base.clone();
                tag_files_base.push(format!("tags/{}/files/", &tag));

                if f.dir_exists(&tag_files_base) {
                    if (!f.create_dirs(&tag_files_base, &args.target_dir, args.test)) {
                        return false;
                    }
                    let sym_files = f.get_files_to_symlink(&tag_files_base);
                    tag_files.push((tag_files_base, sym_files));
                }
            }

            // map destinations to link targets
            // this method allows host-specfic files to take precedence
            // over tag-specific files, which take preference over the others
            // tags are evaluated in the order in which they are supplied
            let mut dests: HashMap<PathBuf, PathBuf> = HashMap::new();
            for file in host_files {
                let dest = args.target_dir.join(
                    file.strip_prefix(&host_files_base).unwrap(),
                );
                dests.insert(dest, file.clone());
            }

            for (tag_base, files) in tag_files {
                for file in files {
                    let dest = args.target_dir.join(
                        file.strip_prefix(&tag_base)
                            .unwrap(),
                    );
                    if !dests.contains_key(&dest) {
                        dests.insert(dest, file.clone());
                    }
                }
            }

            for file in files {
                let dest = args.target_dir.join(
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
            host_post_up_hooks_dir.push(format!("hosts/{}/hooks/post-up/", &args.hostname));
            let mut tag_post_up_hooks_dirs = Vec::new();
            for tag in &args.tags {
                let mut post_up_tag_dir = package_base.clone();
                post_up_tag_dir.push(format!("tags/{}/hooks/post-up/", &tag));
                tag_post_up_hooks_dirs.push(post_up_tag_dir);
            }


            let ok = hooks::run_hooks(&post_up_hooks_dir, &host_post_up_hooks_dir, &tag_post_up_hooks_dirs, args.test);
            if !ok {
                return false;
            }
        }

        return true;
    }

    pub fn uninstall(&self) -> bool {

        let args = self.args;

        let f: FS = FS::new(self.args.force);

        for package1 in &args.packages {
            println!(":: Removing package {:?}", package1);

            let mut package_base = args.dir.clone();
            package_base.push(package1);


            println!(":: Will remove all links in {:?}", args.target_dir);
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

            let mut pre_down_hooks_dir = global_hooks_base.clone();
            pre_down_hooks_dir.push("pre-down");
            let mut host_pre_down_hooks_dir = package_base.clone();
            host_pre_down_hooks_dir.push(format!("hosts/{}/hooks/pre-down/", &args.hostname));
            let mut tag_pre_down_hooks_dirs = Vec::new();
            for tag in &args.tags {
                let mut pre_down_tag_dir = package_base.clone();
                pre_down_tag_dir.push(format!("tags/{}/hooks/pre-down/", &tag));
                tag_pre_down_hooks_dirs.push(pre_down_tag_dir);
            }

            println!(":: Executing pre-down hooks.");
            //println!("{:?}", host_pre_down_hooks_dir);
            let ok = hooks::run_hooks(&pre_down_hooks_dir, &host_pre_down_hooks_dir, &tag_pre_down_hooks_dirs, args.test);

            if !ok {
                return false;
            }

            println!(":: Creating parent dirs where required.");

            let mut global_files_base = package_base.clone();
            global_files_base.push("files");

            f.create_dirs(&global_files_base, &args.target_dir, args.test);
            let files = f.get_files_to_symlink(&global_files_base);

            // host specific config
            let mut host_files_base = package_base.clone();
            host_files_base.push(format!("hosts/{}/files/", &args.hostname));

            let mut host_files: Vec<PathBuf> = vec![];

            if f.dir_exists(&host_files_base) {
                if (!f.create_dirs(&host_files_base, &args.target_dir, args.test)) {
                    return false;
                }

                // symlink the files
                host_files = f.get_files_to_symlink(&host_files_base);
            }

            // tag specific config
            let mut tag_files: Vec<(PathBuf, Vec<PathBuf>)> = vec![];
            for tag in &args.tags {
                let mut tag_files_base = package_base.clone();
                tag_files_base.push("tags");
                tag_files_base.push(&tag);
                tag_files_base.push("files");

                if f.dir_exists(&tag_files_base) {
                    if (!f.create_dirs(&tag_files_base, &args.target_dir, args.test)) {
                        return false;
                    }
                    let sym_files = f.get_files_to_symlink(&tag_files_base);
                    tag_files.push((tag_files_base, sym_files));
                }
            }


            // map destinations to link targets
            // this method allows host-specfic files to take precedence
            // over tag-specific files, which take preference over the others
            // tags are evaluated in the order in which they are supplied
            let mut dests: HashSet<PathBuf> = HashSet::new();
            for file in host_files {
                let dest = args.target_dir.join(
                    file.strip_prefix(&host_files_base).unwrap(),
                );
                dests.insert(dest);
            }

            for (tag_base, files) in tag_files {
                for file in files {
                    let dest = args.target_dir.join(
                        file.strip_prefix(&tag_base)
                            .unwrap(),
                    );
                    if !dests.contains(&dest) {
                        dests.insert(dest);
                    }
                }
            }

            for file in files {
                let dest = args.target_dir.join(
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
            host_post_down_hooks_dir.push(format!("hosts/{}/hooks/post-down", &args.hostname));
            let mut tag_post_down_hooks_dirs = Vec::new();
            for tag in &args.tags {
                let mut post_down_tag_dir = package_base.clone();
                post_down_tag_dir.push(format!("tags/{}/hooks/post-down/", &tag));
                tag_post_down_hooks_dirs.push(post_down_tag_dir);
            }


            let ok = hooks::run_hooks(&post_down_hooks_dir, &host_post_down_hooks_dir, &tag_post_down_hooks_dirs, args.test);
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

        let file_base = match add_args.filename.strip_prefix(&self.args.target_dir) {
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
