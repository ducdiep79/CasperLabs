//! Command line tool for creating a Wasm contract and tests for use on the CasperLabs Platform.

#![deny(warnings)]

use std::{
    env,
    path::{Path, PathBuf},
};

use clap::{crate_version, App, Arg};
use lazy_static::lazy_static;

pub mod common;
mod contract_package;
pub mod dependency;
mod tests_package;

const APP_NAME: &str = "cargo-casperlabs";
const ABOUT: &str =
    "A command line tool for creating a Wasm contract and tests at <path> for use on the \
     CasperLabs Platform.";
const TOOLCHAIN: &str = "nightly-2020-02-14";

const ROOT_PATH_ARG_NAME: &str = "path";
const ROOT_PATH_ARG_VALUE_NAME: &str = "path";
const ROOT_PATH_ARG_HELP: &str = "Path to new folder for contract and tests";

const WORKSPACE_PATH_ARG_NAME: &str = "workspace-path";
const WORKSPACE_PATH_ARG_LONG: &str = "workspace-path";

const FAILURE_EXIT_CODE: i32 = 101;

lazy_static! {
    static ref USAGE: String = format!(
        r#"cargo casperlabs <path>
    rustup install {0}
    rustup target add --toolchain {0} wasm32-unknown-unknown
    cd <path>/tests
    cargo test"#,
        TOOLCHAIN
    );
    static ref ARGS: Args = Args::new();
}

#[derive(Debug)]
struct Args {
    root_path: PathBuf,
    workspace_path: Option<PathBuf>,
}

impl Args {
    fn new() -> Self {
        // If run normally, the args passed are 'cargo-casperlabs', '<target dir>'.  However, if run
        // as a cargo subcommand (i.e. cargo casperlabs <target dir>), then cargo injects a new arg:
        // 'cargo-casperlabs', 'casperlabs', '<target dir>'.  We need to filter this extra arg out.
        //
        // This yields the situation where if the binary receives args of
        // 'cargo-casperlabs', 'casperlabs' then it might be a valid call (not a cargo subcommand -
        // the user entered 'cargo-casperlabs casperlabs' meaning to create a target dir called
        // 'casperlabs') or it might be an invalid call (the user entered 'cargo casperlabs' with no
        // target dir specified).  The latter case is assumed as being more likely.
        let filtered_args_iter = env::args().enumerate().filter_map(|(index, value)| {
            if index == 1 && value.as_str() == "casperlabs" {
                None
            } else {
                Some(value)
            }
        });

        let root_path_arg = Arg::with_name(ROOT_PATH_ARG_NAME)
            .required(true)
            .value_name(ROOT_PATH_ARG_VALUE_NAME)
            .help(ROOT_PATH_ARG_HELP);

        let workspace_path_arg = Arg::with_name(WORKSPACE_PATH_ARG_NAME)
            .long(WORKSPACE_PATH_ARG_LONG)
            .takes_value(true)
            .hidden(true);

        let arg_matches = App::new(APP_NAME)
            .version(crate_version!())
            .about(ABOUT)
            .usage(USAGE.as_str())
            .arg(root_path_arg)
            .arg(workspace_path_arg)
            .get_matches_from(filtered_args_iter);

        let root_path = arg_matches
            .value_of(ROOT_PATH_ARG_NAME)
            .expect("expected path")
            .into();

        let workspace_path = arg_matches
            .value_of(WORKSPACE_PATH_ARG_NAME)
            .map(PathBuf::from);

        Args {
            root_path,
            workspace_path,
        }
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn workspace_path(&self) -> Option<&Path> {
        self.workspace_path.as_deref()
    }
}

fn main() {
    if ARGS.root_path().exists() {
        common::print_error_and_exit(&format!(
            ": destination '{}' already exists",
            ARGS.root_path().display()
        ));
    }

    common::create_dir_all(ARGS.root_path());

    contract_package::run_cargo_new();
    contract_package::update_cargo_toml();
    contract_package::add_rust_toolchain();
    contract_package::replace_main_rs();
    contract_package::add_config();

    tests_package::run_cargo_new();
    tests_package::update_cargo_toml();
    tests_package::add_rust_toolchain();
    tests_package::add_build_rs();
    tests_package::replace_main_rs();
    tests_package::copy_wasm_files();
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::TOOLCHAIN;

    #[test]
    fn check_toolchain_version() {
        let mut toolchain_path = env::current_dir().unwrap().display().to_string();
        let index = toolchain_path
            .find("/execution-engine/")
            .expect("test should be run from within execution-engine workspace");
        toolchain_path.replace_range(index + 18.., "rust-toolchain");

        let toolchain_contents =
            fs::read(&toolchain_path).unwrap_or_else(|_| panic!("should read {}", toolchain_path));
        let expected_toolchain_value = String::from_utf8_lossy(&toolchain_contents)
            .trim()
            .to_string();

        // If this fails, ensure `TOOLCHAIN` is updated to match the value in
        // "execution-engine/rust-toolchain".
        assert_eq!(&*expected_toolchain_value, TOOLCHAIN);
    }
}
