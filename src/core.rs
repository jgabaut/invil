//  SPDX-License-Identifier: GPL-3.0-only
/*  Build tool with support for git tags, wrapping make.
 *  Copyright (C) 2023-2025  jgabaut
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
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use std::time::Instant;
use std::env;
use crate::ops::{do_build, do_run, do_delete, do_query, gen_header};

#[cfg(feature = "anvilPy")]
use crate::anvil_py::{parse_pyproject_toml, AnvilPyEnv};

#[cfg(feature = "anvilCustom")]
use crate::anvil_custom::{parse_anvilcustom_toml, AnvilCustomEnv};

use crate::exit;
use std::cmp::Ordering;
use std::fs::{self, File};
use git2::{Repository, Status, RepositoryInitOptions, ErrorCode};
use is_executable::is_executable;
use toml::Table;
use std::process::ExitCode;
use std::io::{Write, BufReader, BufRead};
use crate::utils::{
    print_grouped_args,
};
use regex::Regex;
use std::fmt;
use std::io;

pub const INVIL_NAME: &str = env!("CARGO_PKG_NAME");
pub const INVIL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INVIL_OS: &str = env::consts::OS;
pub const INVIL_LOG_FILE: &str = "anvil.log";
pub const ANVIL_SOURCE_KEYNAME: &str = "source";
pub const ANVIL_BIN_KEYNAME: &str = "bin";
pub const ANVIL_MAKE_VERS_KEYNAME: &str = "makevers";
pub const ANVIL_AUTOMAKE_VERS_KEYNAME: &str = "automakevers";
pub const ANVIL_TESTSDIR_KEYNAME: &str = "tests";
pub const ANVIL_BUILDS_DIR_KEYNAME: &str = "dir";
pub const ANVIL_BONEDIR_KEYNAME: &str = "testsdir";
pub const ANVIL_KULPODIR_KEYNAME: &str = "errortestsdir";
pub const ANVIL_VERSION_KEYNAME: &str = "version";
pub const ANVIL_KERN_KEYNAME: &str = "kern";
pub const EXPECTED_AMBOSO_API_LEVEL: &str = "2.1.0";
pub const MIN_AMBOSO_V_EXTENSIONS: &str = "2.0.1";
pub const MIN_AMBOSO_V_STEGO_NOFORCE: &str = "2.0.3";
pub const MIN_AMBOSO_V_STEGODIR: &str = "2.0.3";
pub const MIN_AMBOSO_V_KERN: &str = "2.0.2";
pub const MIN_AMBOSO_V_LEGACYPARSE: &str = "1.8.0";
pub const MIN_AMBOSO_V_PYKERN: &str = "2.1.0";
pub const MIN_AMBOSO_V_SKIPRETRYSTEGO: &str = "2.0.4";
pub const MIN_AMBOSO_V_DENY_ANVILPY: &str = "2.0.5";
pub const MIN_AMBOSO_V_CUSTKERN: &str = "2.1.0";
pub const MIN_AMBOSO_V_DENY_ANVILCUST: &str = "2.0.9";
pub const MIN_AMBOSO_V_CHECK_DETACHED: &str = "2.0.11";
pub const ANVIL_INTERPRETER_TAG_REGEX: &str = "stego.lock$";
pub const ANVIL_DEFAULT_CONF_PATH: &str = ".anvil/anvil.toml";
pub const RULELINE_MARK_CHAR: char = '\t';
pub const RULE_REGEX: &str = "^([[:graph:]^:]+:){1,1}([[:space:]]*[[:graph:]]*)*$";
pub const RULEWARN_REGEX: &str = "^ +";

pub enum CutDirection {
    Before,
    After,
}

pub const AMBOSO_BUILD_LEGACY_POS: u64 = 0;
pub const AMBOSO_SOURCE_LEGACY_POS: u64 = 1;
pub const AMBOSO_BIN_LEGACY_POS: u64 = 2;
pub const AMBOSO_MAKEVERS_LEGACY_POS: u64 = 3;
pub const AMBOSO_TESTS_LEGACY_POS: u64 = 4;
pub const AMBOSO_AUTOMAKEVERS_LEGACY_POS: u64 = 5;
pub const AMBOSO_VERSIONS_LEGACY_POS: u64 = 6;
pub const AMBOSO_BONE_LEGACY_POS: u64 = 0;
pub const AMBOSO_KULPO_LEGACY_POS: u64 = 2;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = format!("{} - A simple build tool leveraging make", INVIL_NAME), long_about = format!("{} - A drop-in replacement for amboso", INVIL_NAME), disable_version_flag = true)]
pub struct Args {
    /// Specify the directory to host tags
    #[arg(short = 'D', long, default_value = "./bin", value_name = "BIN_DIR")]
    pub amboso_dir: Option<PathBuf>,

    /// Specify the directory to host stego.lock
    #[arg(short = 'O', long, default_value = ".", value_name = "STEGO_DIR")]
    pub stego_dir: Option<PathBuf>,

    /// Specify the directory to host build
    #[arg(short = 'I', long, default_value = ".", value_name = "BUILDS_DIR")]
    pub builds_dir: Option<PathBuf>,

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

    /// Specify anvil version target
    #[arg(short = 'a', long, value_name = "ANVIL_VERSION", default_value = EXPECTED_AMBOSO_API_LEVEL)]
    pub anvil_version: Option<String>,

    /// Specify anvil kern target
    #[arg(short = 'k', long, value_name = "ANVIL_KERN", default_value = "amboso-C")]
    pub anvil_kern: Option<String>,

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
    #[arg(short = 'J', long, default_value = "false")]
    pub logged: bool,

    /// Disable color output
    #[arg(short = 'P', long, default_value = "false")]
    pub no_color: bool,

    /// Enable force build
    #[arg(short = 'F', long, default_value = "false")]
    pub force: bool,

    /// Disable calling make rebuild
    #[arg(short = 'R', long, default_value = "false")]
    pub no_rebuild: bool,

    /// Pass configuration argument
    #[arg(short = 'C', long, value_name = "CONFIG_ARG", allow_hyphen_values = true)]
    pub config: Option<String>,

    /// Pass CFLAGS argument
    #[arg(short = 'Z', long, value_name = "CFLAGS_ARG", allow_hyphen_values = true)]
    pub cflags: Option<String>,

    /// Disable extensions to amboso 2.0
    #[arg(short = 'e', long, default_value = "false")]
    pub strict: bool,

    //TODO: Handle -C flag for passing start time for recursive calls

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Everything after `--` gets captured here
    #[arg(trailing_var_arg = true)]
    pub extra_args: Vec<String>,
}

#[derive(Debug)]
pub enum AmbosoMode {
    TestMode,
    TestMacro,
    GitMode,
    BaseMode,
}

#[derive(Debug)]
pub enum AmbosoLintMode {
    FullCheck,
    LintOnly,
    Lex,
    NajloFull,
    NajloDebug,
    NajloQuiet,
}

#[derive(Debug, PartialEq)]
pub enum AnvilKern {
    AmbosoC,
    AnvilPy,
    Custom,
}

#[derive(Debug)]
pub enum StegoFormat {
    Toml,
    Legacy,
}

#[derive(Debug)]
pub struct AmbosoEnv {

    /// Anvil version we run as
    pub anvil_version: String,

    /// Enable extensions to amboso 2.0
    pub enable_extensions: bool,

    /// Runmode
    pub run_mode: Option<AmbosoMode>,

    /// Path to stego.lock dir
    pub stego_dir: Option<PathBuf>,

    /// Path to amboso dir from wd
    pub amboso_dir: Option<PathBuf>,

    /// Path to builds dir
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
    pub versions_table: BTreeMap<SemVerKey, String>,

    /// Table with supported versions for base mode and description
    pub basemode_versions_table: BTreeMap<SemVerKey, String>,

    /// Table with supported versions for git mode and description
    pub gitmode_versions_table: BTreeMap<SemVerKey, String>,

    /// String used for configure command argument
    pub configure_arg: String,

    /// String used for CFLAGS
    pub cflags_arg: String,

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

    /// Anvil kern
    pub anvil_kern: AnvilKern,

    /// Optional AnvilPyEnv, only used when anvil_kern is AnvilPy
    #[cfg(feature = "anvilPy")]
    pub anvilpy_env: Option<AnvilPyEnv>,

    /// Optional AnvilCustomEnv, only used when anvil_kern is Custom
    #[cfg(feature = "anvilCustom")]
    pub anvilcustom_env: Option<AnvilCustomEnv>,
}

pub struct AmbosoConf {
    /// Anvil kern
    pub anvil_kern: AnvilKern,

    /// Anvil version we run as
    pub anvil_version: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Run all tests or the passed TESTNAME
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
        /// sets record mode
        #[arg(short, long)]
        build: bool,
        query: Option<String>
    },
    /// Tries building latest tag
    Build,
    /// Prepare a new anvil project
    Init {
        /// picks a specific kern
        #[arg(short, long)]
        kern: Option<String>,
        /// Argument to specify directory to init
        init_dir: Option<PathBuf>,
        template_name: Option<String>,
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
                                debug!("Test: {k}");
                                debug!("Path: {}", v.display());
                            }
                            for (k,v) in env.kulpotests_table.iter() {
                                debug!("Error Test: {k}");
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
                    AmbosoMode::TestMacro => {
                        // Listing all tag names is done later, in do_query
                    }
                    _ => todo!("List flag for {:?} mode", env.run_mode),
                }
            } else if args.list_all {
                for (k, v) in env.versions_table.iter() {
                    info!("Tag: {{{}}}, Desc: {{{}}}", k, v);
                }
            }

            if env.do_build {
                let build_res = do_build(env,args);
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
                let run_res = do_run(env,args);
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
                let delete_res = do_delete(env,args);
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
                            let build_res = do_build(env,&args_copy);
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
                            let build_res = do_build(env,&args_copy);
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
                            let delete_res = do_delete(env,&args_copy);
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
                            let delete_res = do_delete(env,&args_copy);
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

            match env.anvil_kern {
                AnvilKern::AmbosoC => {
                    //By default, run do_query()
                    let query_res = do_query(env,args);
                    match query_res {
                        Ok(s) => {
                            trace!("{}", s);
                        }
                        Err(e) => {
                            error!("do_query() failed in handle_amboso_env(). Err: {}", e);
                            exit(1);
                        }
                    }
                }
                AnvilKern::AnvilPy => {
                    trace!("Skipping do_query() since anvil_kern is anvilPy");
                }
                AnvilKern::Custom => {
                    trace!("Skipping do_query() since anvil_kern is custom");
                }
            }
        }
        None => {
            error!("Invalid: None env.run_mode");
        }
    }
}

fn handle_subcommand(args: &mut Args, env: &mut AmbosoEnv) {
    match &args.command {
        Some(Commands::Test { list, query, build}) => {
            if *build {
                env.do_build = true;
            }
            if *list {
                args.list = true;
            }
            if let Some(q) = query {
                println!("query: {}", q);
                args.test = true;
                args.tag = Some(q.to_string());
                env.run_mode = Some(AmbosoMode::TestMode);
            } else {
                args.testmacro = true;
                env.run_mode = Some(AmbosoMode::TestMacro);
            }
            let query_res = do_query(env,args);
            match query_res {
                Ok(s) => {
                    trace!("{}", s);
                    exit(1);
                }
                Err(e) => {
                    error!("do_query() failed in handle_amboso_env(). Err: {}", e);
                    exit(1);
                }
            }
        }
        Some(Commands::Build) => {
            match env.run_mode {
                Some(AmbosoMode::GitMode) => {
                    let latest_tag = env.gitmode_versions_table.last_key_value(); //.max_by(|a, b| semver_compare(a.unwrap(), b));
                    match latest_tag {
                        Some(lt) => {
                            info!("Latest tag: {}", lt.0);
                            args.tag = Some(lt.0.to_string());
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
                    let latest_tag = env.basemode_versions_table.last_key_value(); //keys().max_by(|a, b| semver_compare(a, b));
                    match latest_tag {
                        Some(lt) => {
                            info!("Latest tag: {}", lt.0);
                            args.tag = Some(lt.0.to_string());
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

fn parse_version_core(version: &str) -> Vec<u64> {
    version
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect()
}

fn parse_version_parts(version: &str) -> (Vec<u64>, String, String) {
    let parts: Vec<&str> = version.splitn(2, '-').collect();
    let version_core = parse_version_core(parts[0]);

    let (pre_release, build) = if parts.len() == 2 {
        let mut subparts = parts[1].splitn(2, '+');
        let pre_release = subparts.next().unwrap_or_default();
        let build = subparts.next().unwrap_or_default();
        (pre_release.to_string(), build.to_string())
    } else {
        (String::new(), String::new())
    };

    (version_core, pre_release, build)
}

pub fn semver_compare(v1: &str, v2: &str) -> Ordering {
    let (version_core1, pre_release1, build1) = parse_version_parts(v1);
    let (version_core2, pre_release2, build2) = parse_version_parts(v2);

    for (a, b) in version_core1.iter().zip(version_core2.iter()) {
        match a.cmp(b) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    // If version cores are equal, compare pre-release metadata
    match (pre_release1.is_empty(), pre_release2.is_empty()) {
        (true, true) => {} // Both are empty, continue
        (true, false) => return Ordering::Greater, // v1 is normal, v2 has pre-release
        (false, true) => return Ordering::Less, // v1 has pre-release, v2 is normal
        (false, false) => match pre_release1.cmp(&pre_release2) {
            Ordering::Equal => {}
            other => return other,
        },
    }

    // If pre-release metadata is equal or both are empty, compare build metadata
    match build1.cmp(&build2) {
        Ordering::Equal => {}
        other => return other,
    }

    // If everything is equal so far, compare lengths
    version_core1.len().cmp(&version_core2.len())
}

pub fn is_git_repo_clean(path: &PathBuf, args: &Args) -> Result<bool, String> {
    // Open the repository
    let repo = Repository::discover(path);

    match repo {
        Ok(r) => {
            // Check if there are any modified files in the working directory
            let statuses = r.statuses(None);
            match statuses {
                Ok(s) => {
                    for entry in s.iter() {
                        match entry.status() {
                            Status::WT_MODIFIED | Status::INDEX_MODIFIED | Status::INDEX_NEW => {
                                // There are uncommitted changes
                                info!("Uncommitted changes:");
                                info!("  {}", entry.path().unwrap());
                                return Ok(false);
                            }
                            Status::WT_NEW => {
                                // Untracked files are ignored, after 0.2.14,
                                // to behave like amboso.
                                debug!("Untracked entry found:");
                                debug!("  {}", entry.path().unwrap());
                            }
                            _ => (),
                        }
                    }

                    // No uncommitted changes
                    Ok(true)
                }
                Err(e) => {
                    error!("Failed getting repo statuses. Err: {e}");
                    Err("Failed repo.statuses()".to_string())
                }
            }
        }
        Err(e) => {
            error!("Failed discover of repo at {{{}}}.", path.display());
            if e.code() == ErrorCode::NotFound {
                error!("Could not find repo.");
                if ! args.strict {
                    //Without --strict, we return success when current directory is not a repo.
                    return Ok(true);
                } else {
                    debug!("is_git_repo_clean():    Strict behaviour, quitting on missing repo");
                }
            }
            Err("Failed repo discovery".to_string())
        }
    }
}


fn check_stego_file(stego_path: &PathBuf, amboso_bin_path: &Path, builds_dir: &PathBuf, format: StegoFormat) -> Result<AmbosoEnv,String> {
    if stego_path.exists() {
        trace!("Found {}", stego_path.display());
        let res = match format {
            StegoFormat::Toml => parse_stego_toml(stego_path, amboso_bin_path, builds_dir),
            StegoFormat::Legacy => parse_legacy_stego(stego_path)
        };
        match res {
            Ok(mut a) => {
                //trace!("Stego contents: {{{:#?}}}", a);
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
                        trace!("Support for test mode is on");
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
                                            } else if is_executable(test_path.clone()) {
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
                                        Err(e) => {
                                            warn!("Error on kulpotests path loop. Err: {e}");
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("Failed reading kulpotests dir. Err: {e}");
                                a.support_testmode = false;
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
                                            } else if is_executable(test_path.clone()) {
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
                                        Err(e) => {
                                            warn!("Error on bonetests path loop. Err: {e}");
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("Failed reading bonetests dir. Err: {e}");
                                a.support_testmode = false;
                            }
                        }
                    }
                }
                match a.builds_dir {
                    Some(ref b) => {
                        trace!("Have builds_dir, value: {{{}}}", b.display());
                    }
                    None => {
                        error!("Missing builds_dir value");
                        return Err("Missing builds_dir value".to_string());
                    }

                };
                Ok(a)
            }
            Err(e) => {
                Err(e)
            }
        }
    } else {
        Err(format!("Can't find {}.", stego_path.display()))
    }
}

pub fn check_amboso_dir(dir: &Path, args: &Args) -> Result<AmbosoEnv,String> {
    if ! dir.exists() {
        if ! args.strict {
            debug!("No amboso_dir found at {{{}}}. Preparing it.", dir.display());
            match fs::create_dir_all(dir) {
                Ok(_) => {
                    debug!("Created amboso dir, proceeding.");
                }
                Err(e) => {
                    error!("Failed creating amboso dir. Err: {e}");
                    return Err("Failed creating amboso dir.".to_string());
                }
            }
        } else {
            debug!("check_amboso_dir():    Strict behaviour, quitting on missing amboso dir.");
            return Err(format!("Can't find {}. Quitting.", dir.display()));
        }
    }


    match semver_compare(&args.anvil_version.clone().expect("Failed initialising anvil_version"), MIN_AMBOSO_V_STEGODIR) {
        Ordering::Less => {
            warn!("Taken legacy path, checking for stego.lock at {{{}}}", dir.display());
            trace!("Found {}", dir.display());
            let mut stego_path = dir.to_path_buf();
            stego_path.push("stego.lock");
            match semver_compare(&args.anvil_version.clone().unwrap(), MIN_AMBOSO_V_LEGACYPARSE) {
                Ordering::Less => {
                    warn!("Trying to parse a legacy format stego.lock at {{{}}}", stego_path.display());
                    check_stego_file(&stego_path, dir, &args.builds_dir.clone().expect("Failed initialising anvil_builds_dir"), StegoFormat::Legacy)
                }
                Ordering::Greater | Ordering::Equal => {
                    check_stego_file(&stego_path, dir, &args.builds_dir.clone().expect("Failed initilising anvil_builds_dir"), StegoFormat::Toml)
                }
            }
        }
        Ordering:: Equal | Ordering::Greater => {
            trace!("Found {}", dir.display());
            let mut stego_path;
            match &args.stego_dir {
                Some(query_dir) => {
                    // We use the provided dir
                    stego_path = query_dir.clone();
                    stego_path.push("stego.lock");
                    let amb_env = check_stego_file(&stego_path, dir, &args.builds_dir.clone().expect("Failed initialising anvil_builds_dir"), StegoFormat::Toml);
                    match amb_env {
                        Ok(a) => {
                            return Ok(a);
                        }
                        Err(e) => {
                            warn!("Failed reading stego.lock at {{{}}}. Err: {e}.", stego_path.display());
                            match semver_compare(&args.anvil_version.clone().expect("Failed initialising anvil_version"), MIN_AMBOSO_V_SKIPRETRYSTEGO) {
                                Ordering::Less => {
                                    warn!("Taken legacy path");
                                    warn!("Will retry to find it at {{{}}}.", dir.display());
                                    stego_path = dir.to_path_buf();
                                    stego_path.push("stego.lock");
                                }
                                Ordering::Equal | Ordering::Greater => {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                None => {
                    // We look into amboso_dir when no stego_dir was passed
                    stego_path = dir.to_path_buf();
                    stego_path.push("stego.lock");
                }
            }
            check_stego_file(&stego_path, dir, &args.builds_dir.clone().expect("Failed initialing anvil_builds_dir"), StegoFormat::Toml)
        }
    }
}

pub fn parse_invil_toml(invil_path: &PathBuf) -> Result<AmbosoConf, String> {
    let start_time = Instant::now();
    debug!("Checking global config file at {}", invil_path.display());
    let invil = fs::read_to_string(invil_path);
    match invil {
        Ok(i) => {
            parse_invil_tomlvalue(&i, start_time)
        },
        Err(e) => {
            error!("Could not read anvil_conf.toml contents");
            Err(e.to_string())
        },
    }
}

fn parse_invil_tomlvalue(invil_str: &str, start_time: Instant) -> Result<AmbosoConf, String> {
    let toml_value = invil_str.parse::<Table>();
    match toml_value {
        Ok(y) => {
            let mut anvil_conf: AmbosoConf = AmbosoConf {
                anvil_version: EXPECTED_AMBOSO_API_LEVEL.to_string(),
                anvil_kern: AnvilKern::AmbosoC,
            };
            if let Some(anvil_table) = y.get("anvil").and_then(|v| v.as_table()) {
                if let Some(anvil_version) = anvil_table.get(ANVIL_VERSION_KEYNAME) {
                    let anvil_v_str = anvil_version.as_str().expect("toml conversion failed");
                    if is_semver(anvil_v_str) {
                        if anvil_v_str.starts_with("2.0") {
                            match anvil_v_str {
                                "2.0.0" => {
                                    info!("Running as 2.0, turning off extensions");
                                    anvil_conf.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "2.0.1" | "2.0.2" | "2.0.3" => {
                                    info!("Running as <2.0.4");
                                    anvil_conf.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "2.0.4" | "2.0.5" | "2.0.6" | "2.0.7" | "2.0.8" | "2.0.9" | "2.0.10" | "2.0.11" | "2.0.12" => {
                                    info!("Running as {{{}}}", anvil_v_str);
                                    anvil_conf.anvil_kern = AnvilKern::AmbosoC;
                                }
                                _ => {
                                    error!("Invalid anvil_version: {{{anvil_version}}}");
                                    return Err("Invalid anvil_version".to_string());
                                }
                            }
                            trace!("ANVIL_VERSION: {{{anvil_version}}}");
                            anvil_conf.anvil_version = anvil_v_str.to_string();
                        } else if anvil_v_str.starts_with("2.1") {
                            trace!("Accepting preview version from stego.lock");
                            match anvil_v_str {
                                "2.1.0" => {
                                    info!("Running as {{{}}}", anvil_v_str);
                                }
                                _ => {
                                    error!("Invalid anvil_version: {{{anvil_version}}}");
                                    return Err("Invalid anvil_version".to_string());
                                }
                            }
                            trace!("ANVIL_VERSION: {{{anvil_version}}}");
                            anvil_conf.anvil_version = anvil_v_str.to_string();
                        } else {
                            error!("Invalid anvil_version: {{{anvil_version}}}");
                            return Err("Invalid anvil_version".to_string());
                        }
                    } else {
                        error!("Invalid anvil_version: {{{}}}", anvil_v_str);
                        return Err("Invalid anvil_version".to_string());
                    }
                } else {
                    debug!("Missing ANVIL_VERSION definition.");
                }

                match semver_compare(&anvil_conf.anvil_version, MIN_AMBOSO_V_KERN) {
                    Ordering::Less => {},
                    Ordering::Equal | Ordering::Greater => {
                        if let Some(anvil_kern) = anvil_table.get(ANVIL_KERN_KEYNAME) {
                            match anvil_kern.as_str().expect("toml conversion failed") {
                                "amboso-C" => {
                                    anvil_conf.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "anvilPy" => {
                                    match semver_compare(&anvil_conf.anvil_version, MIN_AMBOSO_V_PYKERN) {
                                        Ordering::Less => {
                                            error!("Unsupported AnvilKern value: {{{anvil_kern}}}");
                                            warn!("Try running as >={MIN_AMBOSO_V_PYKERN}");
                                            warn!("Current anvil_version: {{{}}}", anvil_conf.anvil_version);
                                            return Err("Unsupported anvil_kern".to_string());
                                        },
                                        Ordering::Equal | Ordering::Greater => {
                                            match semver_compare(&anvil_conf.anvil_version, MIN_AMBOSO_V_DENY_ANVILPY) {
                                                Ordering::Less => {
                                                    return Err("Unsupported anvil_kern".to_string());
                                                }
                                                Ordering::Equal | Ordering::Greater => {
                                                    warn!("The AnvilPy kern is experimental. Be careful.");
                                                    anvil_conf.anvil_kern = AnvilKern::AnvilPy;
                                                }
                                            }
                                        }
                                    }
                                }
                                "custom" => {
                                    match semver_compare(&anvil_conf.anvil_version, MIN_AMBOSO_V_CUSTKERN) {
                                        Ordering::Less => {
                                            error!("Unsupported AnvilKern value: {{{anvil_kern}}}");
                                            warn!("Try running as >={MIN_AMBOSO_V_CUSTKERN}");
                                            warn!("Current anvil_version: {{{}}}", anvil_conf.anvil_version);
                                            return Err("Unsupported anvil_kern".to_string());
                                        },
                                        Ordering::Equal | Ordering::Greater => {
                                            match semver_compare(&anvil_conf.anvil_version, MIN_AMBOSO_V_DENY_ANVILCUST) {
                                                Ordering::Less => {
                                                    return Err("Unsupported anvil_kern".to_string());
                                                }
                                                Ordering::Equal | Ordering::Greater => {
                                                    warn!("The AnvilCustom kern is experimental. Be careful.");
                                                    anvil_conf.anvil_kern = AnvilKern::Custom;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    error!("Invalid AnvilKern value: {{{anvil_kern}}}");
                                    return Err("Invalid anvil_kern".to_string());
                                }
                            }
                        } else {
                            debug!("Missing ANVIL_KERN definition.");
                        }
                    }
                }
            } else {
                debug!("Missing ANVIL section.");
            }
            Ok(anvil_conf)
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done parsing anvil.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed parsing {{{}}} as TOML. Err: [{e}]", invil_str);
            Err("Failed parsing TOML".to_string())
        }
    }
}

pub fn parse_stego_toml(stego_path: &PathBuf, amboso_dir_path: &Path, builds_dir: &PathBuf) -> Result<AmbosoEnv,String> {
    let start_time = Instant::now();
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    //trace!("Stego contents: {{{}}}", stego);
    let mut stego_dir = stego_path.clone();
    if ! stego_dir.pop() {
        error!("Failed pop for {{{}}}", stego_dir.display());
        return Err(format!("Unexpected stego_dir value: {{{}}}", stego_dir.display()));
    }
    if stego_dir.to_str().expect("Could not stringify {stego_path}").is_empty() {
        stego_dir = PathBuf::from(".");
    }
    if stego_dir.exists() {
        trace!("Setting ANVIL_STEGODIR to {{{}}}", stego_dir.display());
    } else {
        error!("Failed setting ANVIL_BINDIR from passed stego_path: {{{}}}", stego_path.display());
        return Err(format!("Could not get stego_dir from {{{}}}", stego_path.display()));
    }
    parse_stego_tomlvalue(&stego, amboso_dir_path, stego_dir, builds_dir.to_path_buf(), start_time)
}

fn parse_stego_tomlvalue(stego_str: &str, amboso_dir_path: &Path, stego_dir: PathBuf, builds_dir: PathBuf, start_time: Instant) -> Result<AmbosoEnv, String> {
    let toml_value = stego_str.parse::<Table>();
    match toml_value {
        Ok(y) => {
            let mut anvil_env: AmbosoEnv = AmbosoEnv {
                run_mode : None,
                amboso_dir: Some(amboso_dir_path.to_path_buf()),
                stego_dir: Some(stego_dir),
                builds_dir: Some(builds_dir),
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
                start_time,
                configure_arg: "".to_string(),
                cflags_arg: "".to_string(),
                anvil_version: EXPECTED_AMBOSO_API_LEVEL.to_string(),
                enable_extensions: true,
                anvil_kern: AnvilKern::AmbosoC,
                #[cfg(feature = "anvilPy")]
                anvilpy_env: None,
                #[cfg(feature = "anvilCustom")]
                anvilcustom_env: None,
            };
            //trace!("Toml value: {{{}}}", y);
            if let Some(anvil_table) = y.get("anvil").and_then(|v| v.as_table()) {
                if let Some(anvil_version) = anvil_table.get(ANVIL_VERSION_KEYNAME) {
                    let anvil_v_str = anvil_version.as_str().expect("toml conversion failed");
                    if is_semver(anvil_v_str) {
                        if anvil_v_str.starts_with("2.0") {
                            match anvil_v_str {
                                "2.0.0" => {
                                    info!("Running as 2.0, turning off extensions");
                                    anvil_env.enable_extensions = false;
                                    anvil_env.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "2.0.1" | "2.0.2" | "2.0.3" => {
                                    info!("Running as <2.0.4");
                                    anvil_env.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "2.0.4" | "2.0.5" | "2.0.6" | "2.0.7" | "2.0.8" | "2.0.9" | "2.0.10" | "2.0.11" | "2.0.12" => {
                                    info!("Running as {{{}}}", anvil_v_str);
                                    anvil_env.anvil_kern = AnvilKern::AmbosoC;
                                }
                                _ => {
                                    error!("Invalid anvil_version: {{{anvil_version}}}");
                                    return Err("Invalid anvil_version".to_string());
                                }
                            }
                            trace!("ANVIL_VERSION: {{{anvil_version}}}");
                            anvil_env.anvil_version = anvil_v_str.to_string();
                        } else if anvil_v_str.starts_with("2.1") {
                            trace!("Accepting preview version from stego.lock");
                            match anvil_v_str {
                                "2.1.0" => {
                                    info!("Running as {{{}}}", anvil_v_str);
                                }
                                _ => {
                                    error!("Invalid anvil_version: {{{anvil_version}}}");
                                    return Err("Invalid anvil_version".to_string());
                                }
                            }
                            trace!("ANVIL_VERSION: {{{anvil_version}}}");
                            anvil_env.anvil_version = anvil_v_str.to_string();
                        } else {
                            error!("Invalid anvil_version: {{{anvil_version}}}");
                            return Err("Invalid anvil_version".to_string());
                        }
                    } else {
                        error!("Invalid anvil_version: {{{}}}", anvil_v_str);
                        return Err("Invalid anvil_version".to_string());
                    }
                } else {
                    debug!("Missing ANVIL_VERSION definition.");
                }

                match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_KERN) {
                    Ordering::Less => {},
                    Ordering::Equal | Ordering::Greater => {
                        if let Some(anvil_kern) = anvil_table.get(ANVIL_KERN_KEYNAME) {
                            match anvil_kern.as_str().expect("toml conversion failed") {
                                "amboso-C" => {
                                    anvil_env.anvil_kern = AnvilKern::AmbosoC;
                                }
                                "anvilPy" => {
                                    match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_PYKERN) {
                                        Ordering::Less => {
                                            error!("Unsupported AnvilKern value: {{{anvil_kern}}}");
                                            warn!("Try running as >={MIN_AMBOSO_V_PYKERN}");
                                            warn!("Current anvil_version: {{{}}}", anvil_env.anvil_version);
                                            return Err("Unsupported anvil_kern".to_string());
                                        },
                                        Ordering::Equal | Ordering::Greater => {
                                            match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_DENY_ANVILPY) {
                                                Ordering::Less => {
                                                    if ! anvil_env.enable_extensions {
                                                        error!("Strict behaviour, refusing anvilPy kern.");
                                                        return Err("Unsupported anvil_kern".to_string());
                                                    }
                                                }
                                                Ordering::Equal | Ordering::Greater => {
                                                    if ! anvil_env.enable_extensions {
                                                        error!("Strict behaviour, refusing anvilPy kern.");
                                                        return Err("Unsupported anvil_kern".to_string());
                                                    }
                                                }
                                            }
                                            warn!("The AnvilPy kern is experimental. Be careful.");
                                            anvil_env.anvil_kern = AnvilKern::AnvilPy;
                                        }
                                    }
                                }
                                "custom" => {
                                    match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_CUSTKERN) {
                                        Ordering::Less => {
                                            error!("Unsupported AnvilKern value: {{{anvil_kern}}}");
                                            warn!("Try running as >={MIN_AMBOSO_V_CUSTKERN}");
                                            warn!("Current anvil_version: {{{}}}", anvil_env.anvil_version);
                                            return Err("Unsupported anvil_kern".to_string());
                                        },
                                        Ordering::Equal | Ordering::Greater => {
                                            match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_DENY_ANVILCUST) {
                                                Ordering::Less => {
                                                    if ! anvil_env.enable_extensions {
                                                        error!("Strict behaviour, refusing anvilCustom kern.");
                                                        return Err("Unsupported anvil_kern".to_string());
                                                    }
                                                }
                                                Ordering::Equal | Ordering::Greater => {
                                                    if ! anvil_env.enable_extensions {
                                                        error!("Strict behaviour, refusing anvilCustom kern.");
                                                        return Err("Unsupported anvil_kern".to_string());
                                                    }
                                                }
                                            }
                                            warn!("The AnvilCustom kern is experimental. Be careful.");
                                            anvil_env.anvil_kern = AnvilKern::Custom;
                                        }
                                    }
                                }
                                _ => {
                                    error!("Invalid AnvilKern value: {{{anvil_kern}}}");
                                    return Err("Invalid anvil_kern".to_string());
                                }
                            }
                        } else {
                            debug!("Missing ANVIL_KERN definition.");
                        }
                    }
                }
            } else {
                debug!("Missing ANVIL section.");
            }
            if let Some(build_table) = y.get("build").and_then(|v| v.as_table()) {
                if let Some(source_name) = build_table.get(ANVIL_SOURCE_KEYNAME) {
                    trace!("ANVIL_SOURCE: {{{source_name}}}");
                    anvil_env.source = Some(source_name.as_str().expect("toml conversion failed").to_string());
                } else {
                    warn!("Missing ANVIL_SOURCE definition.");
                }
                if let Some(binary_name) = build_table.get(ANVIL_BIN_KEYNAME) {
                    trace!("ANVIL_BIN: {{{binary_name}}}");
                    anvil_env.bin = Some(binary_name.as_str().expect("toml conversion failed").to_string());
                } else {
                    warn!("Missing ANVIL_BIN definition.");
                }
                if let Some(anvil_make_vers_tag) = build_table.get(ANVIL_MAKE_VERS_KEYNAME) {
                    trace!("ANVIL_MAKE_VERS: {{{anvil_make_vers_tag}}}");
                    anvil_env.mintag_make = Some(anvil_make_vers_tag.as_str().expect("toml conversion failed").to_string());
                } else {
                    warn!("Missing ANVIL_MAKE_VERS definition.");
                }
                if let Some(anvil_automake_vers_tag) = build_table.get(ANVIL_AUTOMAKE_VERS_KEYNAME) {
                    trace!("ANVIL_AUTOMAKE_VERS: {{{anvil_automake_vers_tag}}}");
                    anvil_env.mintag_automake = Some(anvil_automake_vers_tag.as_str().expect("toml conversion failed").to_string());
                } else {
                    warn!("Missing ANVIL_AUTOMAKE_VERS definition.");
                }
                if let Some(anvil_builds_dir) = build_table.get(ANVIL_BUILDS_DIR_KEYNAME) {
                    trace!("ANVIL_BUILDS_DIR: {{{anvil_builds_dir}}}");
                    anvil_env.builds_dir = Some(anvil_builds_dir.as_str().expect("toml conversion failed").into());
                }
                if let Some(anvil_testsdir) = build_table.get(ANVIL_TESTSDIR_KEYNAME) {
                    trace!("ANVIL_TESTDIR: {{{anvil_testsdir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let testdir_lit = anvil_testsdir.as_str().expect("toml conversion failed").to_string();
                    path.push(testdir_lit);
                    anvil_env.tests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_TESTDIR definition.");
                    anvil_env.support_testmode = false;
                }
            } else {
                warn!("Missing ANVIL_BUILD section.");
            }
            if let Some(tests_table) = y.get("tests").and_then(|v| v.as_table()) {
                if let Some(anvil_bonetests_dir) = tests_table.get(ANVIL_BONEDIR_KEYNAME) {
                    trace!("ANVIL_BONEDIR: {{{anvil_bonetests_dir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let bonetestdir_lit = anvil_bonetests_dir.as_str().expect("toml conversion failed").to_string();
                    path.push(bonetestdir_lit);
                    anvil_env.bonetests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_BONEDIR definition.");
                    anvil_env.support_testmode = false;
                }
                if let Some(anvil_kulpotests_dir) = tests_table.get(ANVIL_KULPODIR_KEYNAME) {
                    trace!("ANVIL_KULPODIR: {{{anvil_kulpotests_dir}}}");
                    let mut path = PathBuf::new();
                    path.push(".");
                    let kulpotestdir_lit = anvil_kulpotests_dir.as_str().expect("toml conversion failed").to_string();
                    path.push(kulpotestdir_lit);
                    anvil_env.kulpotests_dir = Some(path);
                } else {
                    warn!("Missing ANVIL_KULPODIR definition.");
                    anvil_env.support_testmode = false;
                }
            } else {
                warn!("Missing ANVIL_TESTS section.");
                anvil_env.support_testmode = false;
            }
            if let Some(versions_tab) = y.get("versions").and_then(|v| v.as_table()) {
                anvil_env.versions_table = versions_tab.iter().map(|(key, value)| (SemVerKey(key.to_string()), value.as_str().unwrap().to_string()))
                    .collect();
                if anvil_env.versions_table.is_empty() {
                    warn!("versions_table is empty.");
                } else {
                    for (key, value) in anvil_env.versions_table.iter() {
                        if key.to_string().starts_with('B') {
                            let trimmed_key = key.to_string().trim_start_matches('B').to_string();
                            if ! is_semver(&trimmed_key) {
                                error!("Invalid semver key: {{{}}}", trimmed_key);
                                return Err("Invalid semver key".to_string());
                            }
                            let ins_res = anvil_env.basemode_versions_table.insert(SemVerKey(trimmed_key.clone()), value.clone());
                            match ins_res {
                                None => {},
                                Some(old) => {
                                    error!("parse_stego_toml(): A value was already present for key {{{}}} and was replaced. {{{} => {}}}", trimmed_key, old, value);
                                    return Err("Basemode version conflict".to_string());
                                }
                            }
                        } else {
                            if ! is_semver(&key.to_string()) {
                                error!("Invalid semver key: {{{}}}", key);
                                return Err("Invalid semver key".to_string());
                            }
                            let ins_res = anvil_env.gitmode_versions_table.insert(SemVerKey(key.to_string()), value.clone());
                            match ins_res {
                                None => {},
                                Some(old) => {
                                    error!("parse_stego_toml(): A value was already present for key {{{}}} and was replaced. {{{} => {}}}", key, old, value);
                                    return Err("Gitmode version conflict".to_string());
                                }
                            }
                        }
                    }
                }
            } else {
                warn!("Missing ANVIL_VERSIONS section.");
            }
            let elapsed = start_time.elapsed();
            debug!("Done parsing stego.toml. Elapsed: {:.2?}", elapsed);
            Ok(anvil_env)
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done parsing stego.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed parsing {{{}}} as TOML. Err: [{e}]", stego_str);
            Err("Failed parsing TOML".to_string())
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    // Create the destination directory if it doesn't exist
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            // Recurse into subdirectory
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            // Copy file
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

pub fn handle_init_subcommand(kern: Option<String>, init_dir: Option<PathBuf>, template_name: Option<String>, strict: bool) -> ExitCode {
    let anvil_kern;
    match kern.clone().expect("Unset kern").as_str() {
        "amboso-C" => {
            anvil_kern = AnvilKern::AmbosoC;
        }
        "anvilPy" => {
            anvil_kern = AnvilKern::AnvilPy;
        }
        "custom" => {
            anvil_kern = AnvilKern::Custom;
            if template_name.is_none() {
                error!("Missing template name");
                match init_dir {
                    Some(ref d) => {
                        error!("Usage: invil init -k custom {} <TEMPLATE>", d.display());
                    }
                    None => {
                        error!("Missing init_dir argument");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                let user_home_dir = dirs::home_dir();
                match user_home_dir {
                    Some(_) => {},
                    None => {
                        error!("Could not retrieve user's home directory");
                        return ExitCode::FAILURE;
                    }
                }
                let path = PathBuf::from(format!("{}/.anvil/templates/", user_home_dir.expect("Missing user home dir").display()));

                let mut matched = false;
                let template_paths = fs::read_dir(path);
                match template_paths {
                    Ok(p) => {
                        p.for_each(|x| {
                            match x {
                                Ok(d) => {
                                    let curr_path = d.path();
                                    let file_name = curr_path.file_name().expect("Could not get file name");
                                    info!("Found {}", file_name.display());
                                    if let Some(s) = file_name.to_str() {
                                        let s = s.to_string();
                                        if s == template_name.clone().expect("Missing template name") {
                                            matched = true;
                                        }
                                    } else {
                                        error!("Path is not valid UTF-8!");
                                    }
                                }
                                Err(_) => {}
                            }
                        });
                    }
                    Err(_) => {}
                }
                if !matched {
                    error!("Could not find a matching template for {}", template_name.expect("Missing template name"));
                    return ExitCode::FAILURE;
                }
            }
        }
        _ => {
            eprintln!("Unexpected kern in handle_init_subcommand(): {}", kern.expect("Unset kern"));
            return ExitCode::FAILURE;
        }
    }
    match init_dir {
        Some(target) => {
            debug!("Passed dir to init: {}", target.display());
            let init_res = Repository::init_opts(target.clone(),RepositoryInitOptions::new().no_reinit(true));
            match init_res {
                Ok(repo) => {
                    let repo_workdir = repo.workdir().expect("Repo should not be bare");
                    info!("Created git repo at {{{}}}", repo_workdir.display());

                    let mut dir_basename: String;
                    let caps_dir_basename;
                    if strict {
                        debug!("Doing string init, using \"hello_world\" as target bin name");
                        dir_basename = "hello_world".to_string();
                        caps_dir_basename = "HW_".to_string();
                    } else {
                        let dir_basename_osstr = match repo_workdir.file_name() {
                            Some(d) => {
                                d
                            }
                            None => {
                                error!("Failed to get base name for {{{}}}", repo_workdir.display());
                                return ExitCode::FAILURE;
                            }
                        };

                        match dir_basename_osstr.to_str() {
                            Some(s) => {
                                dir_basename = s.to_string();
                            }
                            None => {
                                error!("Failed converting {{{}}} to string. May contain invalid Unicode.", repo_workdir.display());
                                return ExitCode::FAILURE;
                            }
                        }
                        let dir_basename_nodashes = dir_basename.replace("-", "_");
                        dir_basename = dir_basename_nodashes;
                        caps_dir_basename = dir_basename.to_uppercase();
                    }

                    let mut src = target.clone();
                    match anvil_kern {
                        AnvilKern::AmbosoC => {
                            src.push("src");
                        }
                        AnvilKern::AnvilPy => {
                            src.push(&dir_basename);
                        }
                        AnvilKern::Custom => {
                            let user_home_dir = dirs::home_dir();
                            match user_home_dir {
                                Some(_) => {},
                                None => {
                                    error!("Could not retrieve user's home directory");
                                    return ExitCode::FAILURE;
                                }
                            }
                            let src = PathBuf::from(format!("{}/.anvil/templates/{}/", user_home_dir.expect("Missing user home dir").display(), template_name.expect("Missing template name")));
                            let dst = PathBuf::from(format!("{}", dir_basename));
                            match copy_dir_recursive(&src, &dst) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Could not copy {} to {}, error: {}", src.display(), dst.display(), e);
                                    return ExitCode::FAILURE;
                                }
                            }
                            return ExitCode::SUCCESS;
                        }
                    }
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

                    let stego_path = format!("{}/stego.lock", target.display());
                    trace!("Generating stego.lock -  Target path: {{{}}}", stego_path);
                    let output = File::create(stego_path.clone());
                    let stego_string;
                    match anvil_kern {
                        AnvilKern::AmbosoC => {
                            stego_string = format!("[build]\n
source = \"main.c\"\n
bin = \"{}\"\n
makevers = \"0.1.0\"\n
automakevers = \"0.1.0\"\n
tests = \"tests\"\n
[tests]\n
testsdir = \"ok\"\n
errortestsdir = \"errors\"\n
[versions]\n
\"0.1.0\" = \"{}\"\n", dir_basename, dir_basename);
                        }
                        AnvilKern::AnvilPy => {
                            stego_string = format!("[anvil]\n
kern = \"anvilPy\"\n
version = \"{}\"\n
[build]\n
source = \"main.py\"\n
bin = \"{}\"\n
makevers = \"0.1.0\"\n
automakevers = \"0.1.0\"\n
tests = \"tests\"\n
[tests]\n
testsdir = \"ok\"\n
errortestsdir = \"errors\"\n
[versions]\n
\"0.1.0\" = \"{}\"\n", EXPECTED_AMBOSO_API_LEVEL, dir_basename, dir_basename);
                        }
                        AnvilKern::Custom => {
                            todo!("custom kern in stego string gen");
                        }
                    }
                    match output {
                        Ok(mut f) => {
                            let res = write!(f, "{}", stego_string);
                            match res {
                                Ok(_) => {
                                    debug!("Done generating stego.lock file");
                                }
                                Err(e) => {
                                    error!("Failed writing stego.lock file at {{{}}}. Err: {e}", stego_path);
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed opening stego.lock file at {{{}}}. Err: {e}", stego_path);
                            return ExitCode::FAILURE;
                        }
                    }
                    match anvil_kern {
                        AnvilKern::AmbosoC => {
                            let cmain_path = format!("{}/main.c", src.display());
                            trace!("Generating main.c - Target path: {{{}}}", cmain_path);
                            let output = File::create(cmain_path);
                            let main_string = "#include <stdio.h>\nint main(void) {{\n    printf(\"Hello, World!\\n\");\n    return 0;\n}}\n".to_string();
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
                        }
                        AnvilKern::AnvilPy => {
                            let main_path = format!("{}/main.py", src.display());
                            trace!("Generating main.py - Target path: {{{}}}", main_path);
                            let output = File::create(main_path);
                            let main_string = "#!/bin/python3\ndef main():\n    print(\"Hello, World!\");\n".to_string();
                            match output {
                                Ok(mut f) => {
                                    let res = write!(f, "{}", main_string);
                                    match res {
                                        Ok(_) => {
                                            debug!("Done generating main.py file");
                                        }
                                        Err(e) => {
                                            error!("Failed writing main.py Err: {e}");
                                            return ExitCode::FAILURE;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed opening main.py file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                        AnvilKern::Custom => {
                            todo!("custom kern in main gen");
                        }
                    }
                    let gitignore_path = format!("{}/.gitignore", target.display());
                    trace!("Generating .gitignore Target path: {{{}}}", gitignore_path);
                    let output = File::create(gitignore_path);
                    let gitignore_string;
                    match anvil_kern {
                        AnvilKern::AmbosoC => {
                            gitignore_string = format!("# ignore object files\n*.o\n# also explicitly ignore our executable for good measure\n{}\n# also explicitly ignore our windows executable for good measure\n{}.exe\n# also explicitly ignore our debug executable for good measure\n{}_debug\n#We also want to ignore the dotfile dump if we ever use anvil with -c flag\namboso_cfg.dot\n#We want to ignore anvil log file\nanvil.log\n#We want to ignore default anvil build dir\nbin\n# MacOS DS_Store ignoring\n.DS_Store\n# ignore debug log file\ndebug_log.txt\n# ignore files generated by Autotools\nautom4te.cache/\ncompile\nconfig.guess\nconfig.log\nconfig.status\nconfig.sub\nconfigure\ninstall-sh\nmissing\naclocal.m4\nconfigure~\nMakefile\nMakefile.in\n", dir_basename, dir_basename, dir_basename);
                        }
                        AnvilKern::AnvilPy => {
                            gitignore_string = format!("#Generated by amboso v{}\n# ignore dist dir\ndist\n# ignore __pycache__\n__pycache__\n# ignore build dir\nbuild\n# ignore egg info dir\n{}.egg-info\n", EXPECTED_AMBOSO_API_LEVEL, dir_basename);
                        }
                        AnvilKern::Custom => {
                            todo!("custom kern in .gitignore gen");
                        }
                    }

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
                    match anvil_kern {
                        AnvilKern::AmbosoC => {
                            let makefileam_path = format!("{}/Makefile.am", target.display());
                            trace!("Generating Makefile.am - Target path: {{{}}}", makefileam_path);
                            let output = File::create(makefileam_path);
                            let makefileam_string = format!("AUTOMAKE_OPTIONS = foreign\nCFLAGS = @CFLAGS@\nSHELL := /bin/bash\n.ONESHELL:\nMACHINE := $$(uname -m)\nPACK_NAME = $(TARGET)-$(VERSION)-$(OS)-$(MACHINE)\n{}_SOURCES = src/main.c\nLDADD = $({caps_dir_basename}_LDFLAGS)\nAM_LDFLAGS = -O2\nAM_CFLAGS = $({caps_dir_basename}_CFLAGS) -O2 -Werror -Wpedantic -Wall\nif DEBUG_BUILD\nAM_LDFLAGS += -ggdb -O0\nAM_CFLAGS += \nelse\nAM_LDFLAGS += -s\nendif\n%.o: %.c\n	$(CCOMP) -c $(CFLAGS) $(AM_CFLAGS) $< -o $@\n$(TARGET): $({}_SOURCES:.c=.o)\n	@echo -e \"    AM_CFLAGS: [ $(AM_CFLAGS) ]\"\n	@echo -e \"    LDADD: [ $(LDADD) ]\"\n	$(CCOMP) $(CFLAGS) $(AM_CFLAGS) $({}_SOURCES:.c=.o) -o $@ $(LDADD) $(AM_LDFLAGS)\nclean:\n	@echo -en \"Cleaning build artifacts:  \"\n	-rm $(TARGET)\n	-rm src/*.o\n	-rm static/*.o\n	@echo -e \"Done.\"\ncleanob:\n	@echo -en \"Cleaning object build artifacts:  \"\n	-rm src/*.o\n	-rm static/*.o\n	@echo -e \"Done.\"\nanviltest:\n	@echo -en \"Running anvil tests.\"\n	./anvil -tX\n	@echo -e \"Done.\"\nall: $(TARGET)\nrebuild: clean all\n.DEFAULT_GOAL := all\n", dir_basename, dir_basename, dir_basename);
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
                            let configureac_string = format!("# Generated by invil v{INVIL_VERSION}\nAC_INIT([{}], [0.1.0], [email@example.com])\nAM_INIT_AUTOMAKE([foreign -Wall])\nAC_CANONICAL_HOST\nbuild_linux=no\nbuild_windows=no\nbuild_mac=no\necho \"Host os:  $host_os\"\n\nAC_ARG_ENABLE([debug],  [AS_HELP_STRING([--enable-debug], [Enable debug build])],  [enable_debug=$enableval],  [enable_debug=no])\nAM_CONDITIONAL([DEBUG_BUILD], [test \"$enable_debug\" = \"yes\"])\ncase \"${{host_os}}\" in\n\tmingw*)\n\t\techo \"Building for mingw32: [$host_cpu-$host_vendor-$host_os]\"\n\t\tbuild_windows=yes\n\t\tAC_SUBST([{caps_dir_basename}_CFLAGS], [\"-I/usr/x86_64-w64-mingw32/include -static -fstack-protector\"])\n\t\tAC_SUBST([{caps_dir_basename}_LDFLAGS], [\"-L/usr/x86_64-w64-mingw32/lib\"])\n\t\tAC_SUBST([CCOMP], [\"/usr/bin/x86_64-w64-mingw32-gcc\"])\n\t\tAC_SUBST([OS], [\"w64-mingw32\"])\n\t\tAC_SUBST([TARGET], [\"{}.exe\"])\n\t;;\n\tdarwin*)\n\t\tbuild_mac=yes\n\t\techo \"Building for macos: [$host_cpu-$host_vendor-$host_os]\"\n\t\tAC_SUBST([{caps_dir_basename}_CFLAGS], [\"-I/opt/homebrew/opt/ncurses/include\"])\n\t\tAC_SUBST([{caps_dir_basename}_LDFLAGS], [\"-L/opt/homebrew/opt/ncurses/lib\"])\n\t\tAC_SUBST([OS], [\"darwin\"])\n\t\tAC_SUBST([TARGET], [\"{}\"])\n\t;;\n\tlinux*)\n\t\techo \"Building for Linux: [$host_cpu-$host_vendor-$host_os]\"\n\t\tbuild_linux=yes\n\t\tAC_SUBST([{caps_dir_basename}_CFLAGS], [\"\"])\n\t\tAC_SUBST([{caps_dir_basename}_LDFLAGS], [\"\"])\n\t\tAC_SUBST([OS], [\"Linux\"])\n\t\tAC_SUBST([TARGET], [\"{}\"])\n\t;;\nesac\n\nAM_CONDITIONAL([DARWIN_BUILD], [test \"$build_mac\" = \"yes\"])\nAM_CONDITIONAL([WINDOWS_BUILD], [test \"$build_windows\" = \"yes\"])\nAM_CONDITIONAL([LINUX_BUILD], [test \"$build_linux\" = \"yes\"])\n\nAC_ARG_VAR([VERSION], [Version number])\nif test -z \"$VERSION\"; then\n  VERSION=\"0.1.0\"\nfi\nAC_DEFINE_UNQUOTED([VERSION], [\"$VERSION\"], [Version number])\nAC_CHECK_PROGS([CCOMP], [gcc clang])\nAC_CHECK_HEADERS([stdio.h])\nAC_CHECK_FUNCS([malloc calloc])\nAC_CONFIG_FILES([Makefile])\nAC_OUTPUT\n", dir_basename, dir_basename, dir_basename, dir_basename);
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
                                    let amboso_prog_path = PathBuf::from("amboso/amboso");

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
                                                ExitCode::SUCCESS
                                            }
                                            Err(e) => {
                                                error!("Failed symlink for anvil. Err: {e}");
                                                ExitCode::FAILURE
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed repo.submodule() call. Err: {e}");
                                    ExitCode::FAILURE
                                }
                            }
                        }
                        AnvilKern::AnvilPy => {
                            let pyproj_path = format!("{}/pyproject.toml", target.display());
                            trace!("Generating pyproject.toml - Target path: {{{}}}", pyproj_path);
                            let output = File::create(pyproj_path);
                            let pyproj_string = format!("[project]\nname = \"{}\"\nversion = \"0.1.0\"\n[project.scripts]\n{} = \"{}.main:main\"\n[build-system]\nrequires = [\"setuptools>=61.0\"]\nbuild-backend = \"setuptools.build_meta\"", dir_basename, dir_basename, dir_basename);
                            match output {
                                Ok(mut f) => {
                                    let res = write!(f, "{}", pyproj_string);
                                    match res {
                                        Ok(_) => {
                                            debug!("Done generating pyproject.toml file");
                                        }
                                        Err(e) => {
                                            error!("Failed writing pyproject.toml file. Err: {e}");
                                            return ExitCode::FAILURE;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed opening pyproject.toml file. Err: {e}");
                                    return ExitCode::FAILURE;
                                }
                            }
                            ExitCode::SUCCESS
                        }
                        AnvilKern::Custom => {
                            todo!("custom kern for project info file");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed creating git repo at {{{}}}. Err: {e}", target.display());
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            error!("Missing init_dir argument");
            ExitCode::FAILURE
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
        amboso_dir: None,
        stego_dir: None,
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
        start_time,
        configure_arg: "".to_string(),
        cflags_arg: "".to_string(),
        anvil_version: EXPECTED_AMBOSO_API_LEVEL.to_string(),
        enable_extensions: true,
        anvil_kern: AnvilKern::AmbosoC,
        #[cfg(feature = "anvilPy")]
        anvilpy_env: None,
        #[cfg(feature = "anvilCustom")]
        anvilcustom_env: None,
    };

    let mut override_stego_anvil_version = false;

    if let Some(ref x) = args.anvil_version {
        trace!("Passed anvil_version argument: {x}");
        if is_semver(x) {
            if x.starts_with("2.0") {
                match x.as_str() {
                    "2.0.0" => {
                        info!("Running as 2.0, turning off extensions.");
                        args.strict = true;
                        anvil_env.enable_extensions = false;
                        args.anvil_kern = Some(AnvilKern::AmbosoC.to_string());
                    }
                    "2.0.1" | "2.0.2" | "2.0.3" => {
                        info!("Running as {}", x.as_str());
                        args.anvil_kern = Some(AnvilKern::AmbosoC.to_string());
                    }
                    "2.0.4" | "2.0.5" | "2.0.6" | "2.0.7" | "2.0.8" | "2.0.9" | "2.0.10" | "2.0.11" | "2.0.12" => {
                        info!("Running as {}", x.as_str());
                    }
                    _ => {
                        error!("Invalid anvil_version: {{{}}}", x);
                        return Err("Invalid anvil_version".to_string());
                    }
                }
                trace!("ANVIL_VERSION: {{{x}}}");

                match semver_compare(x, MIN_AMBOSO_V_STEGO_NOFORCE) {
                    Ordering::Greater | Ordering::Equal => {
                        override_stego_anvil_version = true;
                    }
                    Ordering::Less => {
                        warn!("Taken legacy path: stego-provided anvil_version always overrides passed one. Query was: {{{}}}", x);
                        override_stego_anvil_version = false;
                    }
                }
            } else if x.starts_with("2.1") {
                match x.as_str() {
                    "2.1.0" => {
                        info!("Running as {}", x.as_str());
                    }
                    _ => {
                        error!("Invalid anvil_version: {{{}}}", x);
                        return Err("Invalid anvil_version".to_string());
                    }
                }
                trace!("ANVIL_VERSION: {{{x}}}");
            } else {
                match semver_compare(x, MIN_AMBOSO_V_LEGACYPARSE) {
                    Ordering::Less => {
                        match semver_compare(x, "1.0.0") {
                            Ordering::Equal | Ordering::Greater => {
                                warn!("Running as legacy 1.x");
                                debug!("Query was: {{{}}}", x);
                            }
                            Ordering::Less => {
                                error!("Invalid anvil_version: {{{}}}", x);
                                return Err("Invalid anvil_version".to_string());
                            }
                        }
                    }
                    _ => {
                        error!("Invalid anvil_version: {{{}}}", x);
                        return Err("Invalid anvil_version".to_string());
                    }
                }
            }
        } else {
            error!("Invalid anvil_version: {{{}}}", x);
            return Err("Invalid anvil_version".to_string());
        }
    }

    match args.strict {
        true => {
            anvil_env.enable_extensions = false;
        }
        false => {
            if semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_EXTENSIONS) == Ordering::Less {
                anvil_env.enable_extensions = false;
            }
        }
    }

    //Default mode is git
    if ! args.base && ! args.test && ! args.testmacro {
        args.git = true;
    }

    print_grouped_args(args);

    if args.ignore_gitcheck || args.base {
        info!("Ignoring git check.");
    } else {
        let gitcheck_res = is_git_repo_clean(&PathBuf::from("./"), args);
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
                error!("Failed git check");
                return Err(e.to_string());
            }
        }
    }

    //Get AmbosoConf
    //
    if !args.strict {
        let user_home_dir = dirs::home_dir();
        match user_home_dir {
            Some(_) => {},
            None => {
                error!("Could not retrieve user's home directory");
                return Err("Could not find $HOME".to_string());
            }
        }
        let mut invil_conf_path = user_home_dir.expect("Failed getting user's home directory");
        invil_conf_path.push(ANVIL_DEFAULT_CONF_PATH);
        if invil_conf_path.exists() {
            let res = parse_invil_toml(&invil_conf_path);
            match res {
                Ok(c) => {
                    match args.anvil_version {
                        Some(_) => {},
                        None => { args.anvil_version = Some(c.anvil_version);},
                    }
                    match args.anvil_kern {
                        Some(_) => {},
                        None => { args.anvil_kern = Some(c.anvil_kern.to_string()); },
                    }
                }
                Err(e) => {
                    error!("Failed parsing anvil config file.");
                    return Err(e.to_string());
                }
            }
        } else {
            debug!("Could not read global conf from {{{}}}", invil_conf_path.display());
        }
    }

    match &args.builds_dir {
        Some(x) => {
            debug!("Builds dir {{{}}}", x.display());
            anvil_env.builds_dir = Some(x.to_path_buf());
        }
        None => {
            debug!("Using default builds_dir: .");
            anvil_env.builds_dir = Some(PathBuf::from("."));
        }
    }

    //Check amboso_dir arg
    match args.amboso_dir {
        Some(ref x) => {
            debug!("Amboso dir {{{}}}", x.display());
            let res = check_amboso_dir(x, args);
            match res {
                Ok(mut a) => {
                    trace!("{:#?}", a);
                    debug!("Check pass: amboso_dir");
                    if override_stego_anvil_version {
                        a.anvil_version = anvil_env.anvil_version;
                    }
                    if let Some(ref p) = a.stego_dir {
                        debug!("{}", format!("stego_dir: {}", p.display()));
                    }
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

    match anvil_env.anvil_kern {
        AnvilKern::AmbosoC => {}
        AnvilKern::AnvilPy => {
            #[cfg(feature = "anvilPy")] {
                let mut skip_pyparse = false;
                match args.strict {
                    true => {
                        match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_PYKERN) {
                            Ordering::Less => {
                                warn!("Strict behaviour for v{}, skipping reading pyproject.toml", anvil_env.anvil_version);
                                skip_pyparse = true;
                            }
                            Ordering::Equal | Ordering::Greater => {}
                        }
                    }
                    false => {}
                }
                if ! skip_pyparse {
                    debug!("Reading pyproject-toml at {{{}}}", anvil_env.stego_dir.clone().expect("Failed initialising stego_dir").display());
                    let mut pyproj_path = anvil_env.stego_dir.clone().expect("Failed initialising stego_dir");
                    pyproj_path.push("pyproject.toml");
                    let anvilpy_env = parse_pyproject_toml(&pyproj_path);
                    match anvilpy_env {
                        Ok(anvilpy_env) => {
                            debug!("Done parse_pyproject_toml()");
                            debug!("{:?}", anvilpy_env);
                            for author in &anvilpy_env.authors {
                                let mut email = "Unspecified";
                                if let Some(em) = &author.email {
                                    email = em;
                                }
                                debug!("Author: {{{}}}, Email: {{{}}}", author.name, email);
                            }
                            for url in &anvilpy_env.urls {
                                debug!("{{{}}}: {{{}}}", url.name, url.link);
                            }
                            if anvilpy_env.build_sys.backend != "setuptools.build_meta" {
                                error!("Unexpected build system: {{{}}}", anvilpy_env.build_sys.backend);
                                return Err("Unexpected build system".to_string());
                            }
                            anvil_env.anvilpy_env = Some(anvilpy_env);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
            #[cfg(not(feature = "anvilPy"))] {
                // Handle AnvilPy case when the feature is not enabled
                error!("AnvilPy kern feature is not enabled");
                return Err("AnvilPy kern feauture is not enabled".to_string());
            }
        }
        AnvilKern::Custom => {
            #[cfg(feature = "anvilCustom")] {
                let mut skip_custparse = false;
                match args.strict {
                    true => {
                        match semver_compare(&anvil_env.anvil_version, MIN_AMBOSO_V_CUSTKERN) {
                            Ordering::Less => {
                                warn!("Strict behaviour for v{}, skipping reading custom builder from stego.lock", anvil_env.anvil_version);
                                skip_custparse = true;
                            }
                            Ordering::Equal | Ordering::Greater => {}
                        }
                    }
                    false => {}
                }
                if ! skip_custparse {
                    debug!("Reading anvil_custombuilder at {{{}}}", anvil_env.stego_dir.clone().expect("Failed initialising stego_dir").display());
                    let mut stego_path = anvil_env.stego_dir.clone().expect("Failed initialising stego_dir");
                    stego_path.push("stego.lock");
                    let anvilcustom_env = parse_anvilcustom_toml(&stego_path);
                    match anvilcustom_env {
                        Ok(anvilcustom_env) => {
                            debug!("Done parse_anvilcustom_toml()");
                            debug!("{:?}", anvilcustom_env);
                            anvil_env.anvilcustom_env = Some(anvilcustom_env);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
            #[cfg(not(feature = "anvilCustom"))] {
                // Handle AnvilCustom case when the feature is not enabled
                error!("AnvilCustom kern feature is not enabled");
                return Err("AnvilCustom kern feauture is not enabled".to_string());
            }
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
                           let res = gen_header(x, anvil_env.anvil_kern, query, binname);
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

    match anvil_env.amboso_dir {
        Some(ref x) => {
            trace!("Anvil_env amboso_dir: {{{}}}", x.display());
            debug!("TODO:    Validate amboso_env and use it to set missing arguments");
        }
        None => {
            error!("Missing amboso_dir. Quitting.");
            return Err("anvil_env.amboso_dir was empty".to_string());
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
                    warn!("Could not find tests dir in {{stego.lock}}.");
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
                    warn!("Could not find a valid maketag arg in {{stego.lock}}.");
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

    if let Some(ref x) = args.cflags {
        trace!("Passed CFLAGS: {{{}}}", x);
        anvil_env.cflags_arg = x.to_string();
    }

    if let Some(ref x) = args.config {
        let mut backcomp_wanted = true;
        let amboso_config_flag_arg_isfile = "AMBOSO_CONFIG_FLAG_ARG_ISFILE";
        match env::var(amboso_config_flag_arg_isfile) {
            Ok(val) => {
                let int_val = val.parse::<i32>();
                match int_val {
                    Ok(v) => {
                        if v == 0 {
                            backcomp_wanted = false;
                        }
                    }
                    Err(e) => {
                        debug!("Failed reading {{{}: {}}}", amboso_config_flag_arg_isfile, e);
                        debug!("Backcomp requested for config flag");
                    }
                }
            },
            Err(e) => {
                debug!("Failed reading {{{}: {}}}", amboso_config_flag_arg_isfile, e);
                debug!("Backcomp requested for config flag");
            }
        }
        if backcomp_wanted {
            let config_read_res = fs::read_to_string(x);
            match config_read_res {
                Ok(config_str) => {
                    trace!("Read config file: {{{}}}", config_str);
                    anvil_env.configure_arg = config_str;
                }
                Err(e) => {
                    error!("Failed reading config file from {{{}}}. Err: {e}", x);
                    return Err("Failed reading config file".to_string());
                }
            }
        } else {
            trace!("Using config arg: {{{}}}", x);
            anvil_env.configure_arg = x.to_string();
        }
    }

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

    Ok(anvil_env)
}

fn is_semver(input: &str) -> bool {
    let full_regex = Regex::new(
        r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$",
    )
    .expect("Failed to create regex");
    let strict_regex = Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$").expect("Failed to create regex");

    if strict_regex.is_match(input) {
        true
    } else {
        if full_regex.is_match(input) {
            error!("Prerelease or build metadata is not allowed in a strict SemVer key.");
            return false;
        }
        false
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SemVerKey(pub String);

impl Ord for SemVerKey {
    fn cmp(&self, other: &Self) -> Ordering {
        semver_compare(&self.0, &other.0)
    }
}

impl PartialOrd for SemVerKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SemVerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for AnvilKern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            AnvilKern::AmbosoC => {
                write!(f, "amboso-C")
            }
            AnvilKern::AnvilPy => {
                write!(f, "anvilPy")
            }
            AnvilKern::Custom => {
                write!(f, "custom")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_semver() {
        assert_eq!(is_semver("1.2.3"), true);
        assert_eq!(is_semver("1.20.3"), true);
        assert_eq!(is_semver("1.2.3-pr2"), false);
        assert_eq!(is_semver("1.2.3-pr2+b123"), false);
        assert_eq!(is_semver("1.2.3+b123"), false);
        assert_eq!(is_semver("01.2.3"), false);
        assert_eq!(is_semver("1.02.3"), false);
        assert_eq!(is_semver("1.2.03"), false);
    }

    #[test]
    fn test_semver_compare() {

        // Test case 1
        assert_eq!(semver_compare("1.2.3", "1.2.4"), Ordering::Less);
        assert_eq!(semver_compare("2.0.0", "1.9.9"), Ordering::Greater);
        assert_eq!(semver_compare("1.2.0", "1.20.9"), Ordering::Less);
        assert_eq!(semver_compare("1.10.0", "1.1.10"), Ordering::Greater);

        // Test case 2: Test with pre-release metadata
        assert_eq!(semver_compare("1.0.0-alpha", "1.0.0-beta"), Ordering::Less);

        // Test case 3: Test with build metadata
        assert_eq!(semver_compare("1.0.0+build123", "1.0.0+build456"), Ordering::Equal);
        assert_eq!(semver_compare("1.0.0+pr123", "1.0.0+pr456"), Ordering::Equal);
        assert_eq!(semver_compare("1.0.0+pr123", "1.0.0+build456"), Ordering::Equal);

        // Test case 4: Test with both pre-release and build metadata
        assert_eq!(semver_compare("1.0.0-pr1+build123", "1.0.0-pr1+build456"), Ordering::Less);
        assert_eq!(semver_compare("1.0.0-pr2+build123", "1.0.0-pr1+build456"), Ordering::Greater);
        assert_eq!(semver_compare("1.0.0-pr2+build456", "1.0.0-pr1+build123"), Ordering::Greater);

        // Test case 5: Test with only version core and some extension
        assert_eq!(semver_compare("1.0.0", "1.0.0-pr1+build456"), Ordering::Greater);
        assert_eq!(semver_compare("1.0.0", "1.0.0+build456"), Ordering::Greater);
        assert_eq!(semver_compare("1.0.0", "1.0.0-patch123"), Ordering::Greater);

    }

}

pub fn lex_stego_toml(stego_path: &PathBuf) -> Result<String,String> {
    let start_time = Instant::now();
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    trace!("Stego contents: {{{}}}", stego);
    let toml_value = stego.parse::<Table>();
    let allow_nonstr_values = false;
    match toml_value {
        Ok(y) => {
            trace!("Toml value: {{{}}}", y);
            for t in y.iter() {
                println!("Scope: {}", t.0);
                if let Some(table) = y.get(t.0).and_then(|v| v.as_table()) {
                    for key in table.keys() {
                        if let Some(val) = table.get(key) {
                            if val.is_str() {
                                println!("Variable: {}_{}, Value: {}", t.0, key, val);
                            } else if allow_nonstr_values {
                                if val.is_array() {
                                    println!("Array: {}_{}, Name: {}", t.0, key, key);
                                    for (i, inner_v) in val.as_array().expect("Failed parsing array").iter().enumerate() {
                                        if inner_v.is_str() {
                                            println!("Arrvalue: {}_{}[{}], Value: {}", t.0, key, i, inner_v);
                                        }
                                    }
                                } else if val.is_table() {
                                    println!("Struct: {}_{}, Name: {}", t.0, key, key);
                                    let tab = val.as_table().expect("Failed parsing table");
                                    for inner_k in tab.keys() {
                                        if let Some(inner_v) = tab.get(inner_k) {
                                            if inner_v.is_str() {
                                                println!("Structvalue: {}_{}_{}, Value: {}", t.0, key, inner_k, inner_v);
                                            }
                                        } else {
                                            error!("Could not parse inner key {inner_k} for table {key}")
                                        }
                                    }
                                }
                            } else {
                                debug!("Value was not a string and was skipped: {val}");
                            }
                        } else {
                            error!("Could not parse {key}");
                        }
                    }
                } else {
                    error!("Could not parse {}", t.0);
                }
                println!("----------------------------");
            }
            let elapsed = start_time.elapsed();
            debug!("Done lexing stego.toml. Elapsed: {:.2?}", elapsed);
            Ok("Lex success".to_string())
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done lexing stego.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed lexing {{{}}} as TOML. Err: [{}]", stego, e);
            Err("Failed lexing TOML".to_string())
        }
    }
}

pub fn cut_line_at_char(line: &str, delimiter: char, direction: CutDirection) -> &str {
    if let Some(index) = line.find(delimiter) {
        match direction {
            CutDirection::After => &line[index + delimiter.len_utf8()..],
            CutDirection::Before => &line[..index],
        }
    } else {
        match direction {
            CutDirection::After => "",
            CutDirection::Before => line,
        }
    }
}

pub fn parse_legacy_stego(stego_path: &PathBuf) -> Result<AmbosoEnv,String> {
    let start_time = Instant::now();
    let mut stego_dir = stego_path.clone();
    if ! stego_dir.pop() {
        error!("Failed pop for {{{}}}", stego_dir.display());
        return Err(format!("Unexpected stego_dir value: {{{}}}", stego_dir.display()));
    }
    if stego_dir.exists() {
        trace!("Setting ANVIL_BINDIR to {{{}}}", stego_dir.display());
    } else {
        error!("Failed setting ANVIL_BINDIR from passed stego_path: {{{}}}", stego_path.display());
        return Err(format!("Could not get stego_dir from {{{}}}", stego_path.display()));
    }

    // Check if the file exists
    if let Ok(file) = File::open(stego_path) {

        let mut cur_line = 0;
        let mut anvil_env: AmbosoEnv = AmbosoEnv {
            run_mode : None,
            amboso_dir: Some(stego_dir),
            stego_dir: None,
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
            start_time,
            configure_arg: "".to_string(),
            cflags_arg: "".to_string(),
            anvil_version: EXPECTED_AMBOSO_API_LEVEL.to_string(),
            enable_extensions: true,
            anvil_kern: AnvilKern::AmbosoC,
            #[cfg(feature = "anvilPy")]
            anvilpy_env: None,
            #[cfg(feature = "anvilCustom")]
            anvilcustom_env: None,
        };

        for line in BufReader::new(file).lines() {
            if let Ok(line_content) = line {
                let comment = cut_line_at_char(&line_content, '#', CutDirection::After).to_string();
                let stripped_line = cut_line_at_char(&line_content, '#', CutDirection::Before).to_string();
                if  cur_line == AMBOSO_BUILD_LEGACY_POS {
                    trace!("Skipping legacy build header -> {{{}}}", stripped_line);
                    cur_line += 1;
                    continue;
                } else if cur_line == AMBOSO_VERSIONS_LEGACY_POS {
                    trace!("Skipping legacy versions header -> {{{}}}", stripped_line);
                    cur_line += 1;
                    continue;
                } else if cur_line == AMBOSO_SOURCE_LEGACY_POS {
                    if stripped_line.is_empty() {
                        error!("Found empty ANVIL_SOURCE definition.");
                        return Err("Invalid ANVIL_SOURCE definition".to_string());
                    }
                    debug!("Found ANVIL_SOURCE legacy def -> {{{}}}", stripped_line);
                    anvil_env.source = Some(stripped_line.clone());
                } else if cur_line == AMBOSO_BIN_LEGACY_POS {
                    if stripped_line.is_empty() {
                        error!("Found empty ANVIL_BIN definition.");
                        return Err("Invalid ANVIL_BIN definition".to_string());
                    }
                    debug!("Found ANVIL_BIN legacy def -> {{{}}}", stripped_line);
                    anvil_env.bin = Some(stripped_line.clone());
                } else if cur_line == AMBOSO_MAKEVERS_LEGACY_POS {
                    if stripped_line.is_empty() {
                        error!("Found empty ANVIL_MAKEVERS definition.");
                        return Err("Invalid ANVIL_MAKEVERS definition".to_string());
                    }
                    debug!("Found ANVIL_MAKEVERS legacy def -> {{{}}}", stripped_line);
                    anvil_env.mintag_make = Some(stripped_line.clone());
                } else if cur_line == AMBOSO_TESTS_LEGACY_POS {
                    if stripped_line.is_empty() {
                        error!("Found empty ANVIL_TESTDIR definition.");
                        return Err("Invalid ANVIL_TESTDIR definition".to_string());
                    }
                    debug!("Found ANVIL_TESTDIR legacy def -> {{{}}}", stripped_line);
                    anvil_env.tests_dir = Some(stripped_line.clone().into());

                    let mut kazoj_path = anvil_env.tests_dir.clone().expect("Failed setting tests_dir");
                    kazoj_path.push("kazoj.lock");

                    let mut cur_kazoj_line = 0;

                    if let Ok(kazoj_file) = File::open(kazoj_path.clone()) {
                        for kazoj_line in BufReader::new(kazoj_file).lines() {
                            if let Ok(kazoj_line_content) = kazoj_line {
                                let _kazoj_comment = cut_line_at_char(&kazoj_line_content, '#', CutDirection::After);
                                let kazoj_stripped_line = cut_line_at_char(&kazoj_line_content, '#', CutDirection::Before);
                                if cur_kazoj_line == AMBOSO_BONE_LEGACY_POS {
                                    trace!("Skipping legacy tests dir header -> {{{}}}", kazoj_stripped_line);
                                    cur_kazoj_line += 1;
                                    continue;
                                } else if cur_kazoj_line == AMBOSO_KULPO_LEGACY_POS {
                                    trace!("Skipping legacy errortests dir header -> {{{}}}", kazoj_stripped_line);
                                    cur_kazoj_line += 1;
                                    continue;
                                } else if cur_kazoj_line == (AMBOSO_BONE_LEGACY_POS +1) {
                                    if kazoj_stripped_line.is_empty() {
                                        error!("Found empty ANVIL_BONEDIR definition.");
                                        return Err("Invalid ANVIL_BONEDIR definition".to_string());
                                    }
                                    debug!("Found legacy tests dir def -> {{{}}}", kazoj_stripped_line);
                                    anvil_env.bonetests_dir = Some(kazoj_stripped_line.into());
                                } else if cur_kazoj_line == (AMBOSO_KULPO_LEGACY_POS +1) {
                                    if kazoj_stripped_line.is_empty() {
                                        error!("Found empty ANVIL_KULPODIR definition.");
                                        return Err("Invalid ANVIL_KULPODIR definition".to_string());
                                    }
                                    debug!("Found legacy errortests dir def -> {{{}}}", kazoj_stripped_line);
                                    anvil_env.kulpotests_dir = Some(kazoj_stripped_line.into());
                                }
                                cur_kazoj_line += 1;
                            } else {
                                error!("Failed getting kazoj_line from {{{}}} -> {{{}}}", kazoj_path.clone().display(), cur_kazoj_line);
                                return Err(format!("Can't read line {{{}}} from {{{}}}.", cur_kazoj_line, kazoj_path.display()));
                            }
                        }
                    } else {
                        warn!("Can't find kazoj.lock, expected at: {{{}}}", kazoj_path.clone().display());
                        return Err(format!("Can't find kazoj.lock at {{{}}}", kazoj_path.display()));
                    }

                } else if cur_line == AMBOSO_AUTOMAKEVERS_LEGACY_POS {
                    if stripped_line.is_empty() {
                        error!("Found empty ANVIL_AUTOMAKEVERS definition.");
                        return Err("Invalid ANVIL_AUTOMAKEVERS definition".to_string());
                    }
                    debug!("Found ANVIL_AUTOMAKEVERS legacy def -> {{{}}}", stripped_line);
                    anvil_env.mintag_automake = Some(stripped_line.clone());
                } else {
                    //TODO
                    //Should check the reference to ensure it's an error to have empty lines between versions, in legacy format?
                    //If so, we should error when it's needed

                    debug!("Found version definition -> {{{}}}", stripped_line);
                    anvil_env.versions_table.insert(SemVerKey(stripped_line), comment);
                }
                trace!("Line: {{{line_content}}}");
                cur_line += 1;
            } else {
                error!("Failed reading line {{{cur_line}}} from {{{}}}", stego_path.display());
                return  Err(format!("Error while reading {{{}}}", stego_path.display()));
            }
        }

        for (key, value) in anvil_env.versions_table.iter() {
            if key.to_string().starts_with('?') {
                let trimmed_key = key.to_string().trim_start_matches('?').to_string();
                if ! is_semver(&trimmed_key) {
                    error!("Invalid semver key: {{{}}}", trimmed_key);
                    return Err("Invalid semver key".to_string());
                }
                let ins_res = anvil_env.basemode_versions_table.insert(SemVerKey(trimmed_key.clone()), value.clone());
                match ins_res {
                    None => {},
                    Some(old) => {
                        error!("parse_legacy_stego(): A value was already present for key {{{}}} and was replaced. {{{} => {}}}", trimmed_key, old, value);
                        return Err("Basemode version conflict".to_string());
                    }
                }
            } else {
                if ! is_semver(&key.to_string()) {
                    error!("Invalid semver key: {{{}}}", key);
                    return Err("Invalid semver key".to_string());
                }
                let ins_res = anvil_env.gitmode_versions_table.insert(SemVerKey(key.to_string()), value.clone());
                match ins_res {
                    None => {},
                    Some(old) => {
                        error!("parse_legacy_stego(): A value was already present for key {{{}}} and was replaced. {{{} => {}}}", key, old, value);
                        return Err("Gitmode version conflict".to_string());
                    }
                }
            }
        }

        Ok(anvil_env)
    } else {
        error!("Failed opening stego.lock at path from {{{}}}", stego_path.display());
        Err(format!("Can't find stego.lock, expected at {{{}}}", stego_path.display()))
    }
}
