//  SPDX-License-Identifier: GPL-3.0-only
/*  Build tool with support for git tags, wrapping make.
 *  Copyright (C) 2023  jgabaut
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, version 3 of the License.
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::time::Instant;
use std::env;
use crate::ops::{do_build, do_run, do_delete, do_query, gen_c_header};
use crate::exit;
use std::cmp::Ordering;
use std::fs::{self, File};
use git2::{Repository, Error, Status, RepositoryInitOptions};
use is_executable::is_executable;
use toml::Table;
use std::process::ExitCode;
use std::io::Write;
use crate::utils::print_grouped_args;

pub const INVIL_NAME: &str = env!("CARGO_PKG_NAME");
pub const INVIL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INVIL_OS: &str = env::consts::OS;
pub const INVIL_LOG_FILE: &str = "invil.log";
pub const ANVIL_SOURCE_KEYNAME: &str = "source";
pub const ANVIL_BIN_KEYNAME: &str = "bin";
pub const ANVIL_MAKE_VERS_KEYNAME: &str = "makevers";
pub const ANVIL_AUTOMAKE_VERS_KEYNAME: &str = "automakevers";
pub const ANVIL_TESTSDIR_KEYNAME: &str = "tests";
pub const ANVIL_BONEDIR_KEYNAME: &str = "testsdir";
pub const ANVIL_KULPODIR_KEYNAME: &str = "errortestsdir";
pub const EXPECTED_AMBOSO_API_LEVEL: &str = "1.9.7";

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = format!("{} - A simple build tool leveraging make", INVIL_NAME), long_about = format!("{} - A drop-in replacement for amboso", INVIL_NAME), disable_version_flag = true)]
pub struct Args {
    /// Specify the directory to host tags
    #[arg(short = 'D', long, default_value = "./bin", value_name = "BIN_DIR")]
    pub amboso_dir: Option<PathBuf>,

    /// Specify the directory to host tests
    #[arg(short = 'K', long, value_name = "TESTS_DIR")]
    pub kazoj_dir: Option<PathBuf>,

    /// Specify the source name
    #[arg(short = 'S', long, value_name = "SOURCE_NAME")]
    pub source: Option<String>,

    /// Specify the target executable name
    #[arg(short = 'E', long, value_name = "EXEC_NAME")]
    pub execname: Option<String>,

    /// Specify min tag using make as build/clean step
    #[arg(short = 'M', long, value_name = "MAKE_MINTAG")]
    pub maketag: Option<String>,

    /// Generate anvil C header for passed dir
    #[arg(short = 'G', long, value_name = "C_HEADER_DIR", conflicts_with_all(["base","test","testmacro", "linter"]))]
    pub gen_c_header: Option<PathBuf>,

    /// Act as stego linter for passed file
    #[arg(short = 'x', long, value_name = "LINT_TARGET", conflicts_with_all(["gen_c_header", "base", "test", "testmacro"]))]
    pub linter: Option<PathBuf>,

    /// Specify test mode
    #[arg(short = 'T', long, default_value = "false", conflicts_with_all(["base", "git", "testmacro", "gen_c_header", "linter", "init"]))]
    pub test: bool,

    /// Specify base mode
    #[arg(short = 'B', long, default_value = "false", conflicts_with_all(["test", "git", "testmacro", "gen_c_header", "linter"]))]
    pub base: bool,

    /// Specify git mode
    #[arg(short = 'g', long, default_value = "false", conflicts_with_all(["test", "base", "testmacro", "gen_c_header", "linter"]))]
    pub git: bool,

    /// Specify test macro mode
    #[arg(short = 't', long, default_value = "false", conflicts_with_all(["test", "git", "base", "gen_c_header", "linter", "init"]))]
    pub testmacro: bool,

    /// Optional tag argument
    pub tag: Option<String>,

    /// Build all tags for current mode
    #[arg(short = 'i', long, default_value = "false", conflicts_with_all(["gen_c_header", "linter"]))]
    pub init: bool,

    /// Delete binaries for all tags for current mode
    #[arg(short = 'p', long, default_value = "false", conflicts_with_all(["delete", "gen_c_header", "linter"]))]
    pub purge: bool,

    /// Delete binary for passed tag
    #[arg(short = 'd', long, default_value = "false", conflicts_with_all(["test", "testmacro", "gen_c_header", "linter"]))]
    pub delete: bool,

    /// Build binary for passed tag
    #[arg(short = 'b', long, default_value = "false", conflicts_with_all(["gen_c_header", "linter"]))]
    pub build: bool,

    /// Run binary for passed tag
    #[arg(short = 'r', long, default_value = "false", conflicts_with_all(["test", "testmacro", "gen_c_header", "linter"]))]
    pub run: bool,

    /// Print supported tags for current mode
    #[arg(short = 'l', long, default_value = "false")]
    pub list: bool,

    /// Print supported tags for all modes
    #[arg(short = 'L', long, default_value = "false")]
    pub list_all: bool,

    /// Less output
    #[arg(short = 'q', long, default_value = "false", conflicts_with_all(["silent", "verbose"]))]
    pub quiet: bool,

    /// Almost no output
    #[arg(short = 's', long, default_value = "false", conflicts_with_all(["quiet", "verbose"]))]
    pub silent: bool,

    /// More output
    #[arg(short = 'V', long, default_value = "3", conflicts_with_all(["quiet", "silent"]))]
    pub verbose: u8,

    /// Report timer
    #[arg(short = 'w', long, default_value = "false")]
    pub watch: bool,

    /// Print current version and quit
    #[arg(short = 'v', long, default_value = "false", conflicts_with_all(["init", "purge", "delete", "test", "testmacro", "run", "gen_c_header"]))]
    pub version: bool,

    /// Print warranty info and quit
    #[arg(short = 'W', long, default_value = "false", conflicts_with_all(["init", "purge", "delete", "test", "testmacro", "run", "gen_c_header"]))]
    pub warranty: bool,

    /// Ignore git mode checks
    #[arg(short = 'X', long, default_value = "false")]
    pub ignore_gitcheck: bool,

    /// Output to log file
    #[arg(long, default_value = "false")]
    pub logged: bool,

    /// Disable color output
    #[arg(long, default_value = "false")]
    pub no_color: bool,

    //TODO: Handle -C flag for passing start time for recursive calls

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug)]
pub enum AmbosoMode {
    TestMode,
    TestMacro,
    GitMode,
    BaseMode,
}

#[derive(Debug)]
pub struct AmbosoEnv {
    ///Runmode
    pub run_mode: Option<AmbosoMode>,

    /// Path to builds dir from wd
    pub builds_dir: Option<PathBuf>,

    /// Path to tests dir from wd
    pub tests_dir: Option<PathBuf>,

    /// Path to success tests dir from wd
    pub bonetests_dir: Option<PathBuf>,

    /// Path to error tests dir from wd
    pub kulpotests_dir: Option<PathBuf>,

    /// Main source name for queried tag
    pub source: Option<String>,

    /// Bin name for queried tag
    pub bin: Option<String>,

    /// First tag supporting make for current project
    pub mintag_make: Option<String>,

    /// First tag supporting automake for current project
    pub mintag_automake: Option<String>,

    /// Table with all supported versions and description
    pub versions_table: BTreeMap<String, String>,

    /// Table with supported versions for base mode and description
    pub basemode_versions_table: BTreeMap<String, String>,

    /// Table with supported versions for git mode and description
    pub gitmode_versions_table: BTreeMap<String, String>,

    /// Allow test mode run
    pub support_testmode: bool,

    /// Table with supported tests
    pub bonetests_table: BTreeMap<String, PathBuf>,

    /// Table with supported error tests
    pub kulpotests_table: BTreeMap<String, PathBuf>,

    /// Do build op
    pub do_build: bool,

    /// Do run op
    pub do_run: bool,

    /// Do delete op
    pub do_delete: bool,

    /// Do init op
    pub do_init: bool,

    /// Do purge op
    pub do_purge: bool,

    /// Allow make builds
    pub support_makemode: bool,

    /// Allow automake builds
    pub support_automakemode: bool,

    /// Start time
    pub start_time: Instant,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// Tries building latest tag
    Build,
    /// Prepare a new anvil project
    Init {
        /// Argument to specify directory to init
        init_dir: Option<PathBuf>,
    },
    /// Prints invil version
    Version,
}

pub fn handle_amboso_env(env: &mut AmbosoEnv, args: &mut Args) {
    handle_subcommand(args, env);
    match env.run_mode {
        Some(ref runmode) => {
            info!("Runmode: {:?}", runmode);
            match runmode {
                    AmbosoMode::TestMode | AmbosoMode::TestMacro => {
                        if !env.support_testmode {
                            error!("Test mode not supported for this project.");
                            return
                        } else {
                            info!("Supported tests: {}", env.bonetests_table.len() + env.kulpotests_table.len());
                            for (k,v) in env.bonetests_table.iter() {
                                info!("Test: {k}");
                                debug!("Path: {}", v.display());
                            }
                            for (k,v) in env.kulpotests_table.iter() {
                                info!("Error Test: {k}");
                                debug!("Path: {}", v.display());
                            }
                        }
                    }
                    _ => (),
            }
            if args.list {
                match runmode {
                    AmbosoMode::GitMode => {
                        for (k, v) in env.gitmode_versions_table.iter() {
                            info!("Tag: {{{}}}, Desc: {{{}}}", k, v);
                        }
                    },
                    AmbosoMode::BaseMode => {
                        for (k, v) in env.basemode_versions_table.iter() {
                            info!("Tag: {{{}}}, Desc: {{{}}}", k, v);
                        }
                    },
                    _ => todo!("List flag for {:?} mode", env.run_mode),
                }
            } else if args.list_all {
                for (k, v) in env.versions_table.iter() {
                    info!("Tag: {{{}}}, Desc: {{{}}}", k, v);
                }
            }

            if env.do_build {
                let build_res = do_build(&env,&args);
                match build_res {
                    Ok(s) => {
                        trace!("{}", s);
                    }
                    Err(e) => {
                        warn!("do_build() failed in handle_amboso_env(). Err: {}", e);
                    }
                }
            }
            if env.do_run {
                let run_res = do_run(&env,&args);
                match run_res {
                    Ok(s) => {
                        trace!("{}", s);
                    }
                    Err(e) => {
                        warn!("do_run() failed in handle_amboso_env(). Err: {}", e);
                    }
                }
            }
            if env.do_delete {
                let delete_res = do_delete(&env,&args);
                match delete_res {
                    Ok(s) => {
                        trace!("{}", s);
                    }
                    Err(e) => {
                        warn!("do_delete() failed in handle_amboso_env(). Err: {}", e);
                    }
                }
            }
            if env.do_init {
                match runmode {
                    AmbosoMode::GitMode => {
                        debug!("Doing init for git mode");
                        let mut args_copy = args.clone();
                        for tag in env.gitmode_versions_table.keys() {
                            args_copy.tag = Some(tag.to_string());
                            let build_res = do_build(&env,&args_copy);
                            match build_res {
                                Ok(s) => {
                                    trace!("{}", s);
                                }
                                Err(e) => {
                                    warn!("do_init(): Build failed for tag {{{}}}. Err: {}", tag,e);
                                }
                            }
                        }
                    }
                    AmbosoMode::BaseMode => {
                        debug!("Doing init for base mode");
                        let mut args_copy = args.clone();
                        for tag in env.basemode_versions_table.keys() {
                            args_copy.tag = Some(tag.to_string());
                            let build_res = do_build(&env,&args_copy);
                            match build_res {
                                Ok(s) => {
                                    trace!("{}", s);
                                }
                                Err(e) => {
                                    warn!("do_init(): Build failed for tag {{{}}}. Err: {}", tag,e);
                                }
                            }
                        }
                    }
                    AmbosoMode::TestMode => {
                        todo!("Init op for test mode");
                    }
                    AmbosoMode::TestMacro => {
                        todo!("Init op for test macro mode");
                    }
                }
            }
            if env.do_purge {
                match runmode {
                    AmbosoMode::GitMode => {
                        debug!("Doing purge for git mode");
                        let mut args_copy = args.clone();
                        for tag in env.gitmode_versions_table.keys() {
                            args_copy.tag = Some(tag.to_string());
                            let delete_res = do_delete(&env,&args_copy);
                            match delete_res {
                                Ok(s) => {
                                    trace!("{}", s);
                                }
                                Err(e) => {
                                    warn!("do_purge(): Delete failed for tag {{{}}}. Err: {}", tag, e);
                                }
                            }
                        }
                    }
                    AmbosoMode::BaseMode => {
                        debug!("Doing purge for base mode");
                        let mut args_copy = args.clone();
                        for tag in env.basemode_versions_table.keys() {
                            args_copy.tag = Some(tag.to_string());
                            let delete_res = do_delete(&env,&args_copy);
                            match delete_res {
                                Ok(s) => {
                                    trace!("{}", s);
                                }
                                Err(e) => {
                                    warn!("do_purge(): Delete failed for tag {{{}}}. Err: {}", tag, e);
                                }
                            }
                        }
                    }
                    AmbosoMode::TestMode => {
                        todo!("Purge op for test mode");
                    }
                    AmbosoMode::TestMacro => {
                        todo!("Purge op for test macro mode");
                    }
                }
            }

            //By default, run do_query()
            let query_res = do_query(&env,&args);
            match query_res {
                Ok(s) => {
                    trace!("{}", s);
                }
                Err(e) => {
                    warn!("do_query() failed in handle_amboso_env(). Err: {}", e);
                }
            }
        }
        None => {
            error!("Invalid: None env.run_mode");
            return;
        }
    }
}

fn handle_subcommand(args: &mut Args, env: &mut AmbosoEnv) {
    match &args.command {
        Some(Commands::Test { list: _}) => {
            todo!("Test command")
        }
        Some(Commands::Build) => {
            match env.run_mode {
                Some(AmbosoMode::GitMode) => {
                    let latest_tag = env.gitmode_versions_table.keys().max_by(|a, b| semver_compare(a, b));
                    match latest_tag {
                        Some(lt) => {
                            info!("Latest tag: {}", lt);
                            args.tag = Some(lt.to_string());
                            let build_res = do_build(env, args);
                            match build_res {
                                Ok(s) => {
                                    info!("Done quick build command. Res: {s}");
                                    exit(0);
                                }
                                Err(e) => {
                                    error!("Failed quick build command. Err: {e}");
                                    exit(1);
                                }
                            }
                        }
                        None => {
                            error!("Could not find latest tag");
                            exit(1);
                        }
                    }
                }
                Some(AmbosoMode::BaseMode) => {
                    let latest_tag = env.basemode_versions_table.keys().max_by(|a, b| semver_compare(a, b));
                    match latest_tag {
                        Some(lt) => {
                            info!("Latest tag: {}", lt);
                            args.tag = Some(lt.to_string());
                            let build_res = do_build(env, args);
                            match build_res {
                                Ok(s) => {
                                    info!("Done quick build command. Res: {s}");
                                    exit(0);
                                }
                                Err(e) => {
                                    error!("Failed quick build command. Err: {e}");
                                    exit(1);
                                }
                            }
                        }
                        None => {
                            error!("Could not find latest tag");
                            exit(1);
                        }
                    }
                }
                Some(AmbosoMode::TestMode) => {
                    todo!("Build command for test mode")
                }
                Some(AmbosoMode::TestMacro) => {
                    todo!("Build command for test macro")
                }
                None => {
                    error!("Missing runmode for build command");
                    exit(0);
                }
            }
        }
        _ => {}
    }
}

pub fn semver_compare(v1: &str, v2: &str) -> std::cmp::Ordering {
    let parse_version = |version: &str| {
        version
            .split('.')
            .filter_map(|s| s.parse::<u64>().ok())
            .collect::<Vec<_>>()
    };

    let version1 = parse_version(v1);
    let version2 = parse_version(v2);

    for (a, b) in version1.iter().zip(version2.iter()) {
        match a.cmp(b) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    version1.len().cmp(&version2.len())
}

pub fn is_git_repo_clean(path: &PathBuf) -> Result<bool, Error> {
    // Open the repository
    let repo = Repository::discover(path)?;

    // Check if there are any modified files in the working directory
    let statuses = repo.statuses(None)?;

    for entry in statuses.iter() {
        match entry.status() {
            Status::WT_MODIFIED | Status::WT_NEW | Status::INDEX_MODIFIED | Status::INDEX_NEW => {
                // There are uncommitted changes
                info!("Uncommitted changes:");
                info!("  {}", entry.path().unwrap());
                return Ok(false);
            }
            _ => (),
        }
    }

    // No uncommitted changes
    Ok(true)
}


pub fn check_amboso_dir(dir: &PathBuf) -> Result<AmbosoEnv,String> {
    if dir.exists() {
        trace!("Found {}", dir.display());
        let mut stego_path = dir.clone();
        stego_path.push("stego.lock");
        if stego_path.exists() {
            trace!("Found {}", stego_path.display());
            let res = parse_stego_toml(&stego_path);
            match res {
                Ok(mut a) => {
                    trace!("Stego contents: {{{:#?}}}", a);
                    if a.support_testmode {
                        match a.bonetests_dir {
                            Some(ref b) => {
                                trace!("Have bonetests_dir, value: {{{}}}", b.display());
                            }
                            None => {
                                error!("Missing bonetests_dir value");
                                return Err("Missing bonetests_dir value".to_string());
                            }

                        };
                        match a.kulpotests_dir {
                            Some(ref k) => {
                                trace!("Have kulpotests_dir, value: {{{}}}", k.display());
                            }
                            None => {
                                error!("Missing kulpotests_dir value");
                                return Err("Missing kulpotests_dir value".to_string());
                            }

                        };
                        match a.tests_dir {
                            Some(ref s) => {
                                trace!("Have tests_dir, value: {{{}}}", s.display());
                            }
                            None => {
                                error!("Missing tests_dir value");
                                return Err("Missing tests_dir value".to_string());
                            }
                        }
                        if a.support_testmode {
                            let kulpotests_path = PathBuf::from(format!("{}/{}",a.tests_dir.as_ref().unwrap().display(),a.kulpotests_dir.as_ref().unwrap().display()));
                            let kulpo_paths = fs::read_dir(kulpotests_path);
                            match kulpo_paths {
                                Ok(p) => {
                                    p.for_each(|x| {
                                        match x {
                                            Ok(d) => {
                                                let test_path = d.path();
                                                if test_path.ends_with(".stderr") {
                                                    trace!("Test stderr file: {{{}}}", test_path.display());
                                                } else if test_path.ends_with(".stdout") {
                                                    trace!("Test stdout file: {{{}}}", test_path.display());
                                                } else {
                                                    if is_executable(test_path.clone()) {
                                                        debug!("Found kulpo test: {{{}}}", test_path.display());
                                                        let test_name = test_path.file_name();
                                                        match test_name {
                                                            Some(t) => {
                                                                a.kulpotests_table.insert(t.to_str().unwrap().to_string(), test_path);
                                                            }
                                                            None => {
                                                                error!("Failed adding test to kulpo map");
                                                            }
                                                        }
                                                    } else {
                                                        debug!("Kulpo test: {{{}}} not executable", test_path.display());
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Error on kulpotests path loop. Err: {e}");
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Failed reading kulpotests dir. Err: {e}");
                                    return Err("Failed reading kulpotests dir".to_string());
                                }
                            }
                            let bonetests_path = PathBuf::from(format!("{}/{}",a.tests_dir.as_ref().unwrap().display(),a.bonetests_dir.as_ref().unwrap().display()));
                            let bone_paths = fs::read_dir(bonetests_path);
                            match bone_paths {
                                Ok(p) => {
                                    p.for_each(|x| {
                                        match x {
                                            Ok(d) => {
                                                let test_path = d.path();
                                                if test_path.ends_with(".stderr") {
                                                    trace!("Test stderr file: {{{}}}", test_path.display());
                                                } else if test_path.ends_with(".stdout") {
                                                    trace!("Test stdout file: {{{}}}", test_path.display());
                                                } else {
                                                    if is_executable(test_path.clone()) {
                                                        debug!("Found bone test: {{{}}}", test_path.display());
                                                        let test_name = test_path.file_name();
                                                        match test_name {
                                                            Some(t) => {
                                                                a.bonetests_table.insert(t.to_str().unwrap().to_string(), test_path);
                                                            }
                                                            None => {
                                                                error!("Failed adding test to bone map");
                                                            }
                                                        }
                                                    } else {
                                                        debug!("Bone test: {{{}}} not executable", test_path.display());
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Error on bonetests path loop. Err: {e}");
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Failed reading bonetests dir. Err: {e}");
                                    return Err("Failed reading bonetests dir".to_string());
                                }
                            }
                        }
                    }
                    return Ok(a);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else {
            return Err(format!("Can't find {}. Quitting.", stego_path.display()));
        }
    } else {
        return Err(format!("Can't find {}. Quitting.", dir.display()));
    }
}

pub fn parse_stego_toml(stego_path: &PathBuf) -> Result<AmbosoEnv,String> {
    let start_time = Instant::now();
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    trace!("Stego contents: {{{}}}", stego);
    let toml_value = stego.parse::<Table>();
    let mut stego_dir = stego_path.clone();
    if ! stego_dir.pop() {
        error!("Failed pop for {{{}}}", stego_dir.display());
        return Err("Unexpected stego_dir value: {{{stego_dir.display()}}}".to_string());
    }
    if stego_dir.exists() {
        trace!("Setting ANVIL_BINDIR to {{{}}}", stego_dir.display());
    } else {
        error!("Failed setting ANVIL_BINDIR from passed stego_path: {{{}}}", stego_path.display());
        return Err("Could not get stego_dir from {{{stego_path.display()}}}".to_string());
    }
    match toml_value {
        Ok(y) => {
            let mut anvil_env: AmbosoEnv = AmbosoEnv {
                run_mode : None,
                builds_dir: Some(stego_dir),
                source : None,
                bin : None,
                mintag_make : None,
                mintag_automake : None,
                tests_dir : None,
                bonetests_dir : None,
                kulpotests_dir : None,
                versions_table: BTreeMap::new(),
                basemode_versions_table: BTreeMap::new(),
                gitmode_versions_table: BTreeMap::new(),
                support_testmode : true,
                bonetests_table: BTreeMap::new(),
                kulpotests_table: BTreeMap::new(),
                support_makemode : true,
                support_automakemode : false,
                do_build : false,
                do_run : false,
                do_delete : false,
                do_init : false,
                do_purge : false,
                start_time: start_time,
            };
            trace!("Toml value: {{{}}}", y);
            if let Some(build_table) = y.get("build").and_then(|v| v.as_table()) {
                if let Some(source_name) = build_table.get(ANVIL_SOURCE_KEYNAME) {
                    trace!("ANVIL_SOURCE: {{{source_name}}}");
                    anvil_env.source = Some(format!("{}", source_name.as_str().expect("toml conversion failed")));
                } else {
                    warn!("Missing ANVIL_SOURCE definition.");
                }
                if let Some(binary_name) = build_table.get(ANVIL_BIN_KEYNAME) {
                    trace!("ANVIL_BIN: {{{binary_name}}}");
                    anvil_env.bin = Some(format!("{}", binary_name.as_str().expect("toml conversion failed")));
                } else {
                    warn!("Missing ANVIL_BIN definition.");
                }
                if let Some(anvil_make_vers_tag) = build_table.get(ANVIL_MAKE_VERS_KEYNAME) {
                    trace!("ANVIL_MAKE_VERS: {{{anvil_make_vers_tag}}}");
                    anvil_env.mintag_make = Some(format!("{}", anvil_make_vers_tag.as_str().expect("toml conversion failed")));
                } else {
                    warn!("Missing ANVIL_MAKE_VERS definition.");
                }
                if let Some(anvil_automake_vers_tag) = build_table.get(ANVIL_AUTOMAKE_VERS_KEYNAME) {
                    trace!("ANVIL_AUTOMAKE_VERS: {{{anvil_automake_vers_tag}}}");
                    anvil_env.mintag_automake = Some(format!("{}", anvil_automake_vers_tag.as_str().expect("toml conversion failed")));
                } else {
                    warn!("Missing ANVIL_AUTOMAKE_VERS definition.");
                }
                if let Some(anvil_testsdir) = build_table.get(ANVIL_TESTSDIR_KEYNAME) {
                    trace!("ANVIL_TESTDIR: {{{anvil_testsdir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let testdir_lit = format!("{}", anvil_testsdir.as_str().expect("toml conversion failed"));
                    path.push(testdir_lit);
                    anvil_env.tests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_TESTDIR definition.");
                }
            } else {
                warn!("Missing ANVIL_BUILD section.");
            }
            if let Some(tests_table) = y.get("tests").and_then(|v| v.as_table()) {
                if let Some(anvil_bonetests_dir) = tests_table.get(ANVIL_BONEDIR_KEYNAME) {
                    trace!("ANVIL_BONEDIR: {{{anvil_bonetests_dir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let bonetestdir_lit = format!("{}", anvil_bonetests_dir.as_str().expect("toml conversion failed"));
                    path.push(bonetestdir_lit);
                    anvil_env.bonetests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_BONEDIR definition.");
                }
                if let Some(anvil_kulpotests_dir) = tests_table.get(ANVIL_KULPODIR_KEYNAME) {
                    trace!("ANVIL_KULPODIR: {{{anvil_kulpotests_dir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let kulpotestdir_lit = format!("{}", anvil_kulpotests_dir.as_str().expect("toml conversion failed"));
                    path.push(kulpotestdir_lit);
                    anvil_env.kulpotests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_KULPODIR definition.");
                }
            } else {
                warn!("Missing ANVIL_TESTS section.");
            }
            if let Some(versions_tab) = y.get("versions").and_then(|v| v.as_table()) {
                anvil_env.versions_table = versions_tab.iter().map(|(key, value)| (key.to_string(), value.as_str().unwrap().to_string()))
                    .collect();
                if anvil_env.versions_table.len() == 0 {
                    warn!("versions_table is empty.");
                } else {
                    for (key, value) in anvil_env.versions_table.iter() {
                        if key.starts_with('-') {
                            let trimmed_key = key.trim_start_matches('-').to_string();
                            anvil_env.basemode_versions_table.insert(trimmed_key, value.clone());
                        } else {
                            anvil_env.gitmode_versions_table.insert(key.clone(), value.clone());
                        }
                    }
                }
            } else {
                warn!("Missing ANVIL_VERSIONS section.");
            }
            let elapsed = start_time.elapsed();
            debug!("Done parsing stego.toml. Elapsed: {:.2?}", elapsed);
            return Ok(anvil_env);
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done parsing stego.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed parsing {{{}}}  as TOML. Err: [{}]", stego, e);
            return Err("Failed parsing TOML".to_string());
        }
    }
}

pub fn handle_init_subcommand(init_dir: Option<PathBuf>) -> ExitCode {
    match init_dir {
        Some(target) => {
            debug!("Passed dir to init: {}", target.display());
            let init_res = Repository::init_opts(target.clone(),RepositoryInitOptions::new().no_reinit(true));
            match init_res {
                Ok(repo) => {
                    info!("Created git repo at {{{}}}", repo.workdir().expect("Repo should not be bare").display());
                    let mut src = target.clone();
                    src.push("src");
                    let mut bin = target.clone();
                    bin.push("bin");
                    let mut stub_vers = bin.clone();
                    stub_vers.push("v0.1.0");
                    let mut tests = target.clone();
                    tests.push("tests");
                    let mut bonetests = tests.clone();
                    bonetests.push("ok");
                    let mut kulpotests = tests.clone();
                    kulpotests.push("errors");
                    match fs::create_dir_all(src.clone()) {
                        Ok(_) => {
                            debug!("Created src dir");
                        }
                        Err(e) => {
                            error!("Failed creating src dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    match fs::create_dir_all(bin.clone()) {
                        Ok(_) => {
                            debug!("Created bin dir");
                        }
                        Err(e) => {
                            error!("Failed creating bin dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    match fs::create_dir_all(stub_vers.clone()) {
                        Ok(_) => {
                            debug!("Created stub_vers dir");
                        }
                        Err(e) => {
                            error!("Failed creating stub_vers dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    match fs::create_dir_all(tests) {
                        Ok(_) => {
                            debug!("Created tests dir");
                        }
                        Err(e) => {
                            error!("Failed creating tests dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    match fs::create_dir_all(bonetests) {
                        Ok(_) => {
                            debug!("Created bonetests dir");
                        }
                        Err(e) => {
                            error!("Failed creating bonetests dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    match fs::create_dir_all(kulpotests) {
                        Ok(_) => {
                            debug!("Created kulpotests dir");
                        }
                        Err(e) => {
                            error!("Failed creating kulpotests dir. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }

                    let stego_path = format!("{}/stego.lock", bin.display());
                    trace!("Generating stego.lock -  Target path: {{{}}}", stego_path);
                    let output = File::create(stego_path);
                    let stego_string = format!("[build]\n
source = \"main.c\"\n
bin = \"hello_world\"\n
makevers = \"0.1.0\"\n
automakevers = \"0.1.0\"\n
tests = \"tests\"\n
[tests]\n
testsdir = \"ok\"\n
errortestsdir = \"errors\"\n
[versions]\n
\"0.1.0\" = \"hello_world\"\n");
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", stego_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating stego.lock file");
                                }
                                Err(e) => {
                                    error!("Failed writing stego.lock file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening stego.lock file. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    let cmain_path = format!("{}/main.c", src.display());
                    trace!("Generating main.c - Target path: {{{}}}", cmain_path);
                    let output = File::create(cmain_path);
                    let main_string = format!("#include <stdio.h>\nint main(void) {{\n    printf(\"Hello, World!\\n\");\n    return 0;\n}}\n");
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", main_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating main.c file");
                                }
                                Err(e) => {
                                    error!("Failed writing main.c Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening main.c file. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    let gitignore_path = format!("{}/.gitignore", target.display());
                    trace!("Generating .gitignore Target path: {{{}}}", gitignore_path);
                    let output = File::create(gitignore_path);
                    let gitignore_string = format!("# ignore object files\n*.o\n# also explicitly ignore our executable for good measure\nhello_world\n# also explicitly ignore our windows executable for good measure\nhello_world.exe\n# also explicitly ignore our debug executable for good measure\nhello_world_debug\n#We also want to ignore the dotfile dump if we ever use anvil with -c flag\namboso_cfg.dot\n#We want to ignore invil log file\ninvil.log\n# MacOS DS_Store ignoring\n.DS_Store\n# ignore debug log file\ndebug_log.txt\n# ignore files generated by Autotools\nautom4te.cache/\ncompile\nconfig.guess\nconfig.log\nconfig.status\nconfig.sub\nconfigure\ninstall-sh\nmissing\naclocal.m4\nconfigure~\nMakefile\nMakefile.in\n");
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", gitignore_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating .gitignore file");
                                }
                                Err(e) => {
                                    error!("Failed writing .gitignore file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening .gitignore file. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    let makefileam_path = format!("{}/Makefile.am", target.display());
                    trace!("Generating Makefile.am - Target path: {{{}}}", makefileam_path);
                    let output = File::create(makefileam_path);
                    let makefileam_string = format!("AUTOMAKE_OPTIONS = foreign\nCFLAGS = @CFLAGS@\nSHELL := /bin/bash\n.ONESHELL:\nMACHINE := $$(uname -m)\nPACK_NAME = $(TARGET)-$(VERSION)-$(OS)-$(MACHINE)\nhello_world_SOURCES = src/main.c\nLDADD = $(HW_LDFLAGS)\nAM_LDFLAGS = -O2\nAM_CFLAGS = $(HW_CFLAGS) -O2 -Werror -Wpedantic -Wall\nif DEBUG_BUILD\nAM_LDFLAGS += -ggdb -O0\nAM_CFLAGS += \"\"\nelse\nAM_LDFLAGS += -s\nendif\n%.o: %.c\n	$(CCOMP) -c $(CFLAGS) $(AM_CFLAGS) $< -o $@\n$(TARGET): $(hello_world_SOURCES:.c=.o)\n	@echo -e \"    AM_CFLAGS: [ $(AM_CFLAGS) ]\"\n	@echo -e \"    LDADD: [ $(LDADD) ]\"\n	$(CCOMP) $(CFLAGS) $(AM_CFLAGS) $(hello_world_SOURCES:.c=.o) -o $@ $(LDADD) $(AM_LDFLAGS)\nclean:\n	@echo -en \"Cleaning build artifacts:  \"\n	-rm $(TARGET)\n	-rm src/*.o\n	-rm static/*.o\n	@echo -e \"Done.\"\ncleanob:\n	@echo -en \"Cleaning object build artifacts:  \"\n	-rm src/*.o\n	-rm static/*.o\n	@echo -e \"Done.\"\nanviltest:\n	@echo -en \"Running anvil tests.\"\n	./anvil -tX\n	@echo -e \"Done.\"\nall: $(TARGET)\nrebuild: clean all\n.DEFAULT_GOAL := all\n");
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", makefileam_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating Makefile.am file");
                                }
                                Err(e) => {
                                    error!("Failed writing Makefile.am file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening Makefile.am file. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    let configureac_path = format!("{}/configure.ac", target.display());
                    trace!("Generating configure.ac - Target path: {{{}}}", configureac_path);
                    let output = File::create(configureac_path);
                    let configureac_string = format!("AC_INIT([hello_world], [0.1.0], [email@example.com])\nAM_INIT_AUTOMAKE([foreign -Wall])\nAC_CANONICAL_HOST\necho \"Host os:  $host_os\"\nAM_CONDITIONAL([OS_DARWIN], [test \"$host_os\" = \"darwin\"])\nAM_CONDITIONAL([MINGW32_BUILD], [test \"$host_os\" = \"mingw32\"])\nAC_ARG_ENABLE([debug],  [AS_HELP_STRING([--enable-debug], [Enable debug build])],  [enable_debug=$enableval],  [enable_debug=no])\nAM_CONDITIONAL([DEBUG_BUILD], [test \"$enable_debug\" = \"yes\"])\nif test \"$host_os\" = \"mingw32\"; then\n  echo \"Building for mingw32: [$host_cpu-$host_vendor-$host_os]\"\n  AC_SUBST([HW_CFLAGS], [\"-I/usr/x86_64-w64-mingw32/include -static -fstack-protector -DMINGW32_BUILD\"])\n  AC_SUBST([HW_LDFLAGS], [\"-L/usr/x86_64-w64-mingw32/lib\"])\n  AC_SUBST([CCOMP], [\"/usr/bin/x86_64-w64-mingw32-gcc\"])\n  AC_SUBST([OS], [\"w64-mingw32\"])\n  AC_SUBST([TARGET], [\"hello_world.exe\"])\nfi\nif test \"$host_os\" = \"darwin\"; then\n  echo \"Building for macos: [$host_cpu-$host_vendor-$host_os]\"\n  AC_SUBST([HW_CFLAGS], [\"-I/opt/homebrew/opt/ncurses/include\"])\n  AC_SUBST([HW_LDFLAGS], [\"-L/opt/homebrew/opt/ncurses/lib\"])\n  AC_SUBST([OS], [\"darwin\"])\n  AC_SUBST([TARGET], [\"hello_world\"])\nfi\nif test \"$host_os\" = \"linux-gnu\"; then\n  echo \"Building for Linux: [$host_cpu-$host_vendor-$host_os]\"\n  AC_SUBST([HW_CFLAGS], [\"\"])\n  AC_SUBST([HW_LDFLAGS], [\"\"])\n  AC_SUBST([OS], [\"Linux\"])\n  AC_SUBST([TARGET], [\"hello_world\"])\nfi\nAC_ARG_VAR([VERSION], [Version number])\nif test -z \"$VERSION\"; then\n  VERSION=\"0.3.11\"\nfi\nAC_DEFINE_UNQUOTED([VERSION], [\"$VERSION\"], [Version number])\nAC_CHECK_PROGS([CCOMP], [gcc clang])\nAC_CHECK_HEADERS([stdio.h])\nAC_CHECK_FUNCS([malloc calloc])\nAC_CONFIG_FILES([Makefile])\nAC_OUTPUT\n");
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", configureac_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating configure.ac file");
                                }
                                Err(e) => {
                                    error!("Failed writing configure.ac file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening configure.ac file. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    let amboso_path = PathBuf::from("amboso");
                    let amboso_submodule = repo.submodule(
                        "https://github.com/jgabaut/amboso.git",
                        &amboso_path,
                        false
                    );
                    match amboso_submodule {
                        Ok(mut subm) => {
                            debug!("Success on repo.submodule()");
                            let subm_repo = subm.open();
                            match subm_repo {
                                Ok(_) => {
                                    let clone_res = subm.clone(None);
                                    match clone_res {
                                        Ok(sr) => {
                                            info!("Cloned amboso submodule at {{{}}}", sr.workdir().expect("Repo should not be bare").display());
                                            match subm.add_finalize() {
                                                Ok(_) => {
                                                    debug!("Finalised amboso submodule add");
                                                }
                                                Err(e) => {
                                                    error!("Failed finalising amboso submodule. Err: {e}");
                                                    return ExitCode::FAILURE;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed cloning amboso submodule. Err: {e}");
                                            return ExitCode::FAILURE;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed opening amboso submodule repo. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }

                            let mut anvil_path = target.clone();
                            anvil_path.push("anvil");
                            let mut amboso_prog_path = target.clone();
                            amboso_prog_path.push("amboso/amboso");

                            if cfg!(target_os = "windows") {
                                todo!("Support windows symlink");
                                /*
                                 *let ln_res = std::os::windows::fs::symlink_file(amboso_prog_path.clone(), anvil_path.clone());
                                 *match ln_res {
                                 *    Ok(_) => {
                                 *        info!("Symlinked {{{}}} -> {{{}}}", amboso_prog_path.display(), anvil_path.display());
                                 *        return ExitCode::SUCCESS;
                                 *    }
                                 *    Err(e) => {
                                 *        error!("Failed symlink for anvil. Err: {e}");
                                 *        return ExitCode::FAILURE;
                                 *    }
                                 *}
                                 */
                            } else {
                                let ln_res = std::os::unix::fs::symlink(amboso_prog_path.clone(), anvil_path.clone());
                                match ln_res {
                                    Ok(_) => {
                                        info!("Symlinked {{{}}} -> {{{}}}", amboso_prog_path.display(), anvil_path.display());
                                        return ExitCode::SUCCESS;
                                    }
                                    Err(e) => {
                                        error!("Failed symlink for anvil. Err: {e}");
                                        return ExitCode::FAILURE;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed repo.submodule() call. Err: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed creating git repo at {{{}}}. Err: {e}", target.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        None => {
            error!("Missing init_dir argument");
            return ExitCode::FAILURE;
        }
    }
}

pub fn check_passed_args(args: &mut Args) -> Result<AmbosoEnv,String> {

    let start_time = Instant::now();

    match args.logged {
        false => {

        }
        true => {
            trace!("Doing a logged run");
        }
    }

    let mut anvil_env: AmbosoEnv = AmbosoEnv {
        run_mode : None,
        builds_dir: None,
        source : None,
        bin : None,
        mintag_make : None,
        mintag_automake : None,
        tests_dir : None,
        bonetests_dir : None,
        kulpotests_dir : None,
        versions_table: BTreeMap::new(),
        basemode_versions_table: BTreeMap::new(),
        gitmode_versions_table: BTreeMap::new(),
        support_testmode : true,
        bonetests_table: BTreeMap::new(),
        kulpotests_table: BTreeMap::new(),
        support_makemode : true,
        support_automakemode : false,
        do_build : false,
        do_run : false,
        do_delete : false,
        do_init : false,
        do_purge : false,
        start_time: start_time,
    };

    match args.linter {
        Some(ref x) => {
            info!("Linter for file: {{{}}}", x.display());
            if x.exists() {
                trace!("Found {}", x.display());
                let res = parse_stego_toml(x);
                match res {
                    Ok(_) => {
                        info!("Lint successful for {{{}}}.", x.display());
                        return Ok(anvil_env);
                    }
                    Err(e) => {
                        error!("Failed lint for {{{}}}.\nError was:    {e}",x.display());
                        return Err(e);
                    }
                }
            } else {
                error!("Could not find file: {{{}}}", x.display());
                return Err("Failed linter call".to_string());
            }
        }
        None => {
            trace!("-x not asserted.");
        }
    }

    //Default mode is git
    if ! args.base && ! args.test && ! args.testmacro {
        args.git = true;
    }

    print_grouped_args(&args);

    if args.ignore_gitcheck || args.base {
        info!("Ignoring git check.");
    } else {
        let gitcheck_res = is_git_repo_clean(&PathBuf::from("./"));
        match gitcheck_res {
            Ok(s) => {
                if s {
                    debug!("Repo is clean.");
                } else {
                    warn!("Repo has uncommitted changes.");
                    return Err("Dirty repo with git mode on".to_string());
                }
            }
            Err(e) => {
                error!("Failed git check. Error was: {{{}}}", e);
                return Err(e.to_string());
            }
        }
    }

    //Check amboso_dir arg
    match args.amboso_dir {
        Some(ref x) => {
            debug!("Amboso dir {{{}}}", x.display());
            let res = check_amboso_dir(x);
            match res {
                Ok(a) => {
                    trace!("{:#?}", a);
                    debug!("Check pass: amboso_dir");
                    anvil_env = a;
                }
                Err(e) => {
                    error!("Check fail: {e}");
                    return Err(e);
                }
            }
        }
        None => {
            error!("Missing amboso dir argument. Quitting.");
            return Err("Missing amboso_dir arg".to_string());
        }
    }

    match args.gen_c_header {
        Some(ref x) => {
            match args.tag {
                Some (ref query) => {
                    debug!("TODO: check if query is not a valid tag?");
                    match anvil_env.bin {
                        Some (ref binname) => {
                           info!("Generating C header for {{{}}} to dir: {{{}}}", query, x.display());
                           let res = gen_c_header(x, query, binname);
                            match res {
                                Ok(_) => {
                                    info!("C header gen successful for {{{}}}.", query);
                                    exit(0);
                                }
                                Err(e) => {
                                    error!("C header gen failed for {{{}}}.\nError was:    {e}", query);
                                    return Err(e);
                                }
                            }
                        }
                        None => {
                            error!("Missing bin name for C header gen mode");
                            return Err("Missing bin name for C header gen".to_string());
                        }
                    }
                }
                None => {
                    error!("Missing query tag for C header gen mode");
                    return Err("Missing query tag for C header gen".to_string());
                }
            }
        }
        None => {
            trace!("-G not asserted.");
        }
    }

    match anvil_env.builds_dir {
        Some(ref x) => {
            trace!("Anvil_env builds_dir: {{{}}}", x.display());
            debug!("TODO:    Validate amboso_env and use it to set missing arguments");
        }
        None => {
            error!("Missing builds_dir. Quitting.");
            return Err("anvil_env.builds_dir was empty".to_string());
        }
    }

    match args.kazoj_dir {
        Some(ref x) => {
            debug!("Tests dir {{{}}}", x.display());
            if x.exists() {
                debug!("{} exists", x.display());
                anvil_env.tests_dir = Some(x.clone());
            }
            debug!("TODO:    Validate kazoj_dir");
        }
        None => {
            trace!("Missing tests dir. Checking if stego.lock had a valid tests_dir path");
            match anvil_env.tests_dir {
                Some(ref x) => {
                    if x.exists() {
                        debug!("{} exists", x.display());
                        args.kazoj_dir = Some(x.clone());
                        debug!("TODO:    Validate kazoj_dir");
                    } else {
                        warn!("stego.lock tests dir {{{}}} was invalid", x.display());
                        args.kazoj_dir = Some(PathBuf::from("./kazoj"));
                        if args.kazoj_dir.as_ref().unwrap().exists() {
                            debug!("{} exists", args.kazoj_dir.as_ref().unwrap().display());
                            debug!("TODO:    Validate kazoj_dir");
                            anvil_env.tests_dir = args.kazoj_dir.clone();
                        } else {
                            warn!("Could not find test directory, test mode not supported.");
                            anvil_env.support_testmode = false;
                        }
                    }
                }
                None => {
                    warn!("stego.lock had no tests dir");
                    args.kazoj_dir = Some(PathBuf::from("./kazoj"));
                    if args.kazoj_dir.as_ref().unwrap().exists() {
                        debug!("{} exists", args.kazoj_dir.as_ref().unwrap().display());
                        debug!("TODO:    Validate kazoj_dir");
                        anvil_env.tests_dir = args.kazoj_dir.clone();
                    } else {
                        warn!("Could not find test directory, test mode not supported.");
                        anvil_env.support_testmode = false;
                    }
                }
            }
        }
    }

    let testmode_support_text = match anvil_env.support_testmode {
        true => "Test mode is supported",
        false => "Test mode is not supported",
    };
    trace!("{}", testmode_support_text);

    match args.source {
        Some(ref x) => {
            debug!("Source {{{}}}", x);
            anvil_env.source = args.source.clone();
            debug!("TODO:  Validate source")
        }
        None => {
            trace!("Missing source arg. Checking if stego.lock had a valid source value");
            match anvil_env.source {
                Some( ref x) => {
                    args.source = Some(x.clone());
                }
                None => {
                    error!("stego.lock did not have a valid source arg. Quitting.");
                    return Err("Could not determine anvil_env.source".to_string());
                }
            }
            debug!("TODO:  Validate source")
        }
    }

    match &args.execname {
        Some(x) => {
            debug!("Execname {{{}}}", x);
            anvil_env.bin = Some(x.to_string());
            debug!("TODO:  Validate execname")
        }
        None => {
            trace!("Missing execname arg. Checking if stego.lock had a valid bin value");
            match anvil_env.bin {
                Some(ref x) => {
                    args.execname = Some(x.clone());
                }
                None => {
                    error!("stego.lock did not have a valid bin arg. Quitting.");
                    return Err("Could not determine anvil_env.bin arg".to_string());
                }
            }
            debug!("TODO:  Validate execname")
        }
    }

    match &args.maketag {
        Some(x) => {
            debug!("Maketag {{{}}}", x);
            anvil_env.mintag_make = Some(x.to_string());
            debug!("TODO:  Validate maketag")
        }
        None => {
            trace!("Missing maketag arg. Checking if stego.lock had a valid bin value");
            match anvil_env.mintag_make {
                Some( ref x) => {
                    args.maketag = Some(x.clone());
                    match anvil_env.mintag_automake {
                        Some ( ref automake_tag ) => {
                            debug!("TODO:  Validate automaketag {}", automake_tag);
                            anvil_env.support_automakemode = true;
                        }
                        None => {
                            warn!("stego.lock did not have a valid automaketag arg.");
                            anvil_env.support_automakemode = false;
                        }
                    }
                }
                None => {
                    warn!("stego.lock did not have a valid maketag arg.");
                    anvil_env.support_makemode = false;
                }
            }
        }
    }
    let makemode_support_text = match anvil_env.support_makemode {
        true => "Make mode is supported",
        false => "Make mode is not supported",
    };
    trace!("{}", makemode_support_text);

    debug!("TODO: check if supported tags can be associated with a directory");

    if args.git {
        anvil_env.run_mode = Some(AmbosoMode::GitMode);
    } else if args.base {
        anvil_env.run_mode = Some(AmbosoMode::BaseMode);
    } else if args.test {
        anvil_env.run_mode = Some(AmbosoMode::TestMode);
    } else if args.testmacro {
        anvil_env.run_mode = Some(AmbosoMode::TestMacro);
    } else {
        panic!("No mode flag was asserted");
    }

    anvil_env.do_build = args.build;
    anvil_env.do_run = args.run;
    anvil_env.do_delete = args.delete;
    anvil_env.do_init = args.init;
    anvil_env.do_purge = args.purge;

    return Ok(anvil_env);
}
