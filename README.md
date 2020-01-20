# scm
Copyright © 2020 Edd Salkield, 2017 Samuel Walladge

Simple configuration file (dotfile) manager written in Rust.
Aims to be as simple as possible, with additional functionality non-core depending on the `scm` base.

[![Build Status](https://travis-ci.org/swalladge/dotfiles-manager.svg?branch=master)](https://travis-ci.org/swalladge/dotfiles-manager)
[![Coverage Status](https://coveralls.io/repos/github/swalladge/dotfiles-manager/badge.svg?branch=master)](https://coveralls.io/github/swalladge/dotfiles-manager?branch=master)

# About

`scm` aims to be a fast, stable, and portable configuration manager.  It is designed to systematically symlink files across your filesystem, with minimal bloat.

GitHub issues are currently used for tracking progress and documenting requirements.

See the the man page for full usage documentation.
View [plain text version here](man/dotfiles-manager.1.txt) or use `man`:

```shell
$ man ./man/dotfiles-manager.1
```

# Installing

## Binary release

Simply download the archive containing the binary for your platform from the
[releases](https://github.com/swalladge/dotfiles-manager/releases) page.
Extract the archive, move the binary to somewhere on your path, and voila!

## From source/git

```shell
$ git clone https://github.com/swalladge/dotfiles-manager
$ cd dotfiles-manager
$ make release
```

This will download the sources and produce a binary at `./target/release/dotfiles-manager`.

## Via cargo

If you have `cargo` and the rust toolchain installed, you can install it through that.

```
$ cargo install dotfiles-manager
```


# Testing

For continuous integration, all tests are run on every push with Travis.

Unit tests are run by cargo with `cargo test`. These are tests on individual
functions and small functionality within each module, created as standard in
rust. ([Rust book reference on testing](https://doc.rust-lang.org/stable/book/second-edition/ch11-01-writing-tests.html))

Integration tests are for testing the program as a whole. They are important
for this software, since its core functionality is working with files and
links; many side effects. These are implemented with custom bash scripts. Run
as follows:

```
$ ./integration_tests.sh

$ # or for a single test:
$ ./integration_tests.sh test_no_force_install.sh

$ # or to disable kcov (useful for local testing - faster)
$ ./integration_tests.sh --no-kcov
$ ./integration_tests.sh --no-kcov test_no_force_install.sh
```

Each test must a bash script in the `./test/integration_tests/`. The main bash
script runner runs each test by sourcing the bash script and calling the
`run_test` function. All code should be inside the `run_test` function and
locally scoped to avoid clashing with the main script or other tests.
The tests get some variables to use:

- `exe`           | the binary to run for the dotfiles manager (with kcov)
- `exe_sans`      | the binary to run for the dotfiles manager (sans kcov)
- `$TEMP_LOCAL`   | the local directory to do stuff in - make files, etc - reset after each test
- `$BASE_DIR`     | root directory of project

Basically, for anything new, there should be unit tests that cover any
functions that don't have IO side effects, and integration tests for any
functionality that interacts with std{in,out,err} or files/link/directories.


# Code coverage

Code coverage reports are generated with `kcov` and uploaded to the [Coveralls
project](https://coveralls.io/github/swalladge/dotfiles-manager).

They can be generated and viewed locally with `make coverage`.


# Makefile

For convenience, a `Makefile` is included with some shortcut commands:

- `clean`: cleans the build directory
- `build`: builds the software
- `coverage`: generates the code coverage report (time consuming)
- `unit-test`: runs the unit tests
- `integration-test`: runs the integration tests
- `test`: runs all tests
- `cloc`: shows a report on lines of code (requires the `cloc` program installed)


# Features

- [x] link files from a directory (package) stow-style
- [x] remove links from a package
- [x] host specific config
- [x] run scripts as hooks before and after installing/removing
- [x] force install/remove links
- [x] run in test mode (no filesystem changes)
- [x] integration tests
- [x] confirm before taking action


# Usage

```
dotfiles-manager 0.0.1
Samuel Walladge <samuel@swalladge.id.au>
Manages dotfiles

USAGE:
    dotfiles-manager [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -f, --force      Force creating/removing directories and symlinks, overwriting any that exist.
    -h, --help       Prints help information
    -y, --yes        Assume yes answer to all prompts - non-interactive mode, useful for scripts.
    -n, --no         Do not actually make any filesystem changes or run hooks
    -V, --version    Prints version information
    -v, --verbose    Be verbose

OPTIONS:
    -d, --dir <DIR>          Set source/repo directory to DIR (default is current dir)
    -B, --hostname <NAME>    override computer's hostname to NAME
    -t, --target <DIR>       Set target base directory to DIR (default is $HOME)

SUBCOMMANDS:
    add        add a file to package
    help       Prints this message or the help of the given subcommand(s)
    install    install tags/packages
    remove     remove tags/packages
```


# License

    dotfiles-manager - experimental dotfiles manager in rust
    Copyright (C) 2017 Samuel Walladge

    This program is free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License as published by the Free
    Software Foundation, either version 3 of the License, or (at your option)
    any later version.

    This program is distributed in the hope that it will be useful, but
    WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
    for more details.

    You should have received a copy of the GNU General Public License along
    with this program.  If not, see <http://www.gnu.org/licenses/>.
