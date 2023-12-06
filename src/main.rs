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
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::{env, fs};
#[macro_use] extern crate log;
use simplelog::*;
use toml::Table;
use git2::{Repository, Error, Status};
use std::collections::BTreeMap;
use std::process::{ExitCode, Command, exit};
use std::io::{self, Write};
use std::fs::File;

const INVIL_VERSION: &str = env!("CARGO_PKG_VERSION");
const INVIL_NAME: &str = env!("CARGO_PKG_NAME");
const INVIL_LOG_FILE: &str = "invil.log";
const ANVIL_SOURCE_KEYNAME: &str = "source";
const ANVIL_BIN_KEYNAME: &str = "bin";
const ANVIL_MAKE_VERS_KEYNAME: &str = "makevers";
const ANVIL_AUTOMAKE_VERS_KEYNAME: &str = "automakevers";
const ANVIL_TESTSDIR_KEYNAME: &str = "tests";
const ANVIL_BONEDIR_KEYNAME: &str = "testsdir";
const ANVIL_KULPODIR_KEYNAME: &str = "errortestsdir";


#[derive(Parser, Debug, Clone)]
#[command(author, version, about = format!("{} - A simple build tool leveraging make", INVIL_NAME), long_about = format!("{} - A drop-in replacement for amboso", INVIL_NAME), disable_version_flag = true)]
struct Args {
    /// Specify the directory to host tags
    #[arg(short = 'D', long, default_value = "./bin", value_name = "BIN_DIR")]
    amboso_dir: Option<PathBuf>,

    /// Specify the directory to host tests
    #[arg(short = 'K', long, value_name = "TESTS_DIR")]
    kazoj_dir: Option<PathBuf>,

    /// Specify the source name
    #[arg(short = 'S', long, value_name = "SOURCE_NAME")]
    source: Option<String>,

    /// Specify the target executable name
    #[arg(short = 'E', long, value_name = "EXEC_NAME")]
    execname: Option<String>,

    /// Specify min tag using make as build/clean step
    #[arg(short = 'M', long, value_name = "MAKE_MINTAG")]
    maketag: Option<String>,

    /// Generate anvil C header for passed dir
    #[arg(short = 'G', long, value_name = "C_HEADER_DIR", conflicts_with_all(["base","test","testmacro", "linter"]))]
    gen_c_header: Option<PathBuf>,

    /// Act as stego linter for passed file
    #[arg(short = 'x', long, value_name = "LINT_TARGET", conflicts_with_all(["gen_c_header", "base", "test", "testmacro"]))]
    linter: Option<PathBuf>,

    /// Specify test mode
    #[arg(short = 'T', long, default_value = "false", conflicts_with_all(["base", "git", "testmacro", "gen_c_header", "linter"]))]
    test: bool,

    /// Specify base mode
    #[arg(short = 'B', long, default_value = "false", conflicts_with_all(["test", "git", "testmacro", "gen_c_header", "linter"]))]
    base: bool,

    /// Specify git mode
    #[arg(short = 'g', long, default_value = "false", conflicts_with_all(["test", "base", "testmacro", "gen_c_header", "linter"]))]
    git: bool,

    /// Specify test macro mode
    #[arg(short = 't', long, default_value = "false", conflicts_with_all(["test", "git", "base", "gen_c_header", "linter"]))]
    testmacro: bool,

    /// Optional tag argument
    tag: Option<String>,

    /// Build all tags for current mode
    #[arg(short = 'i', long, default_value = "false", conflicts_with_all(["gen_c_header", "linter"]))]
    init: bool,

    /// Delete binaries for all tags for current mode
    #[arg(short = 'p', long, default_value = "false", conflicts_with_all(["delete", "gen_c_header", "linter"]))]
    purge: bool,

    /// Delete binary for passed tag
    #[arg(short = 'd', long, default_value = "false", conflicts_with_all(["test", "testmacro", "gen_c_header", "linter"]))]
    delete: bool,

    /// Build binary for passed tag
    #[arg(short = 'b', long, default_value = "false", conflicts_with_all(["gen_c_header", "linter"]))]
    build: bool,

    /// Run binary for passed tag
    #[arg(short = 'r', long, default_value = "false", conflicts_with_all(["test", "testmacro", "gen_c_header", "linter"]))]
    run: bool,

    /// Print supported tags for current mode
    #[arg(short = 'l', long, default_value = "false")]
    list: bool,

    /// Print supported tags for all modes
    #[arg(short = 'L', long, default_value = "false")]
    list_all: bool,

    /// Less output
    #[arg(short = 'q', long, default_value = "false", conflicts_with_all(["silent", "verbose"]))]
    quiet: bool,

    /// Almost no output
    #[arg(short = 's', long, default_value = "false", conflicts_with_all(["quiet", "verbose"]))]
    silent: bool,

    /// More output
    #[arg(short = 'V', long, default_value = "3", conflicts_with_all(["quiet", "silent"]))]
    verbose: u8,

    /// Report timer
    #[arg(short = 'w', long, default_value = "false")]
    watch: bool,

    /// Print current version and quit
    #[arg(short = 'v', long, default_value = "false", conflicts_with_all(["init", "purge", "delete", "test", "testmacro", "run", "gen_c_header"]))]
    version: bool,

    /// Print warranty info and quit
    #[arg(short = 'W', long, default_value = "false", conflicts_with_all(["init", "purge", "delete", "test", "testmacro", "run", "gen_c_header"]))]
    warranty: bool,

    /// Ignore git mode checks
    #[arg(short = 'X', long, default_value = "false")]
    ignore_gitcheck: bool,

    /// Output to log file
    #[arg(long, default_value = "false")]
    logged: bool,

    //TODO: Handle -C flag for passing start time for recursive calls

    /// Subcommand
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug)]
enum AmbosoMode {
    TestMode,
    TestMacro,
    GitMode,
    BaseMode,
}

#[derive(Debug)]
struct AmbosoEnv {
    ///Runmode
    run_mode: Option<AmbosoMode>,

    /// Path to builds dir from wd
    builds_dir: Option<PathBuf>,

    /// Path to tests dir from wd
    tests_dir: Option<PathBuf>,

    /// Path to success tests dir from wd
    bonetests_dir: Option<PathBuf>,

    /// Path to error tests dir from wd
    kulpotests_dir: Option<PathBuf>,

    /// Main source name for queried tag
    source: Option<String>,

    /// Bin name for queried tag
    bin: Option<String>,

    /// First tag supporting make for current project
    mintag_make: Option<String>,

    /// First tag supporting automake for current project
    mintag_automake: Option<String>,

    /// Table with all supported versions and description
    versions_table: BTreeMap<String, String>,

    /// Table with supported versions for base mode and description
    basemode_versions_table: BTreeMap<String, String>,

    /// Table with supported versions for git mode and description
    gitmode_versions_table: BTreeMap<String, String>,

    /// Allow test mode run
    support_testmode: bool,

    /// Do build op
    do_build: bool,

    /// Do run op
    do_run: bool,

    /// Do delete op
    do_delete: bool,

    /// Do init op
    do_init: bool,

    /// Do purge op
    do_purge: bool,

    /// Allow make builds
    support_makemode: bool,

    /// Allow automake builds
    support_automakemode: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
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
    }
}

fn prog_name() -> Option<String> {
    env::current_exe().ok()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
}

fn print_config_args(args: &Args) {
    //Handle config flags
    let mut config_string: String = "".to_owned();
    let amboso_dir_string: String = "D".to_owned();
    let kazoj_dir_string: String = "K".to_owned();
    let source_string: String = "S".to_owned();
    let execname_string: String = "E".to_owned();
    let maketag_string: String = "M".to_owned();
    let ignore_gitcheck_string: String = "X".to_owned();
    match args.amboso_dir {
        Some(ref x) => {
            debug!("Passed amboso_dir: {{{}}}", x.display());
            config_string.push_str(&amboso_dir_string);
        }
        None => {}
    }
    match args.kazoj_dir {
        Some(ref x) => {
            debug!("Passed kazoj_dir: {{{}}}", x.display());
            config_string.push_str(&kazoj_dir_string);
        }
        None => {}
    }
    match args.source {
        Some(ref x) => {
            debug!("Passed source: {{{}}}", x);
            config_string.push_str(&source_string);
        }
        None => {}
    }
    match args.execname {
        Some(ref x) => {
            debug!("Passed execname: {{{}}}", x);
            config_string.push_str(&execname_string);
        }
        None => {}
    }
    match args.maketag {
        Some(ref x) => {
            debug!("Passed maketag: {{{}}}", x);
            config_string.push_str(&maketag_string);
        }
        None => {}
    }
    if args.ignore_gitcheck {
        debug!("Ignore git check is on.");
        config_string.push_str(&ignore_gitcheck_string);
    }
    debug!("Config flags: {{-{}}}", config_string);
}

fn print_mode_args(args: &Args) {
    //Handle mode flags
    let mut flags_string: String = "".to_owned();
    let gitmode_string: String = "g".to_owned();
    let testmode_string: String = "t".to_owned();
    let basemode_string: String = "b".to_owned();
    let testmacromode_string: String = "y".to_owned();
    let gen_c_mode_string: String = "G".to_owned();
    let linter_mode_string: String = "x".to_owned();
    if args.git {
        flags_string.push_str(&gitmode_string);
    }
    if args.test {
        flags_string.push_str(&testmode_string);
    }
    if args.base {
        flags_string.push_str(&basemode_string);
    }
    if args.testmacro {
        flags_string.push_str(&testmacromode_string);
    }
    match args.gen_c_header {
        Some(_) => {
            flags_string.push_str(&gen_c_mode_string);
        }
        None => {
        }
    }
    match args.linter {
        Some(_) => {
            flags_string.push_str(&linter_mode_string);
        }
        None => {
        }
    }
    debug!("Mode flags: {{-{}}}", flags_string);
}

fn print_subcommand_args(args: &Args) {
    match &args.command {
        Some(Commands::Test { list }) => {
            if *list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        }
        Some(Commands::Build) => {
            todo!("Quick build command")
        }
        Some(Commands::Init { init_dir }) => {
            if init_dir.is_some() {
                debug!("Passed dir to init: {}", init_dir.as_ref().expect("Missing init_dir").display());
            } else {
                warn!("Missing init_dir arg for init command.");
                //init_dir = &Some(PathBuf::from("."));
                //debug!("Set . as init_dir");
            }
            todo!("Quick init command")
        }
        None => {}
    }
}

fn print_info_args(args: &Args) {
    //Handle info flags
    let mut info_flags_string: String = "".to_owned();

    if args.version {
        info_flags_string.push_str("v");
    }
    if args.watch {
        info_flags_string.push_str("w");
    }
    if args.quiet {
        info_flags_string.push_str("q");
    }
    if args.silent {
        info_flags_string.push_str("s");
    }
    if args.list {
        info_flags_string.push_str("l");
    }
    if args.list_all {
        info_flags_string.push_str("L");
    }
    if args.warranty {
        info_flags_string.push_str("W");
    }

    debug!("Info flags: {{-{}}}", info_flags_string);
}

fn print_op_args(args: &Args) {
    //Handle op flags
    let mut op_flags_string: String = "".to_owned();

    if args.build {
        op_flags_string.push_str("b");
    }
    if args.run {
        op_flags_string.push_str("r");
    }
    if args.delete {
        op_flags_string.push_str("d");
    }
    if args.init {
        op_flags_string.push_str("i");
    }
    if args.purge {
        op_flags_string.push_str("p");
    }

    debug!("Op flags: {{-{}}}", op_flags_string);
}


fn print_grouped_args(args: &Args) {
    // Log asserted flags
    print_subcommand_args(&args);
    print_config_args(&args);
    print_mode_args(&args);
    print_info_args(&args);
    print_op_args(&args);
}

fn is_git_repo_clean(path: &PathBuf) -> Result<bool, Error> {
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

fn check_amboso_dir(dir: &PathBuf) -> Result<AmbosoEnv,String> {
    if dir.exists() {
        trace!("Found {}", dir.display());
        let mut stego_path = dir.clone();
        stego_path.push("stego.lock");
        if stego_path.exists() {
            trace!("Found {}", stego_path.display());
            let res = parse_stego_toml(&stego_path);
            match res {
                Ok(a) => {
                    trace!("Stego contents: {{{:#?}}}", a);
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

fn parse_stego_toml(stego_path: &PathBuf) -> Result<AmbosoEnv,String> {
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
                support_makemode : true,
                support_automakemode : false,
                do_build : false,
                do_run : false,
                do_delete : false,
                do_init : false,
                do_purge : false,
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
            return Ok(anvil_env);
        }
        Err(e) => {
            error!("Failed parsing {{{}}}  as TOML. Err: [{}]", stego, e);
            return Err("Failed parsing TOML".to_string());
        }
    }
}

fn gen_c_header(target_path: &PathBuf, _target_tag: &String, bin_name: &String) -> Result<String,String> {
    let header_path = format!("{}/anvil__{}.h", target_path.display(), bin_name);
    let output = File::create(header_path);
    let header_string = format!("invil - TODO - C header for {{{}}}", bin_name);
    match output {
        Ok(mut f) => {
            let res = write!(f, "{}", header_string);
            match res {
                Ok(_) => {
                    debug!("Done generating header file");
                }
                Err(e) => {
                    error!("Failed printing header file");
                    return Err(e.to_string());
                }
            }
        }
        Err(_) => {
            return Err("Failed gen of header file".to_string());
        }
    }
    let c_impl_path = format!("{}/anvil__{}.c", target_path.display(), bin_name);
    let output = File::create(c_impl_path);
    let c_impl_string = format!("invil - TODO - C impl for {{{}}}", bin_name);
    match output {
        Ok(mut f) => {
            let res = write!(f, "{}", c_impl_string);
            match res {
                Ok(_) => {
                    debug!("Done generating c impl file");
                }
                Err(e) => {
                    error!("Failed printing c impl file");
                    return Err(e.to_string());
                }
            }
        }
        Err(_) => {
            return Err("Failed gen of c impl file".to_string());
        }
    }
    Ok("Done C generationg".to_string())
}

fn check_passed_args(args: &mut Args) -> Result<AmbosoEnv,String> {

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
        support_makemode : true,
        support_automakemode : false,
        do_build : false,
        do_run : false,
        do_delete : false,
        do_init : false,
        do_purge : false,
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

    if args.ignore_gitcheck || ! args.git{
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

fn print_warranty_info() {
    println!("  THERE IS NO WARRANTY FOR THE PROGRAM, TO THE EXTENT PERMITTED BY
  APPLICABLE LAW.  EXCEPT WHEN OTHERWISE STATED IN WRITING THE COPYRIGHT
  HOLDERS AND/OR OTHER PARTIES PROVIDE THE PROGRAM \"AS IS\" WITHOUT WARRANTY
  OF ANY KIND, EITHER EXPRESSED OR IMPLIED, INCLUDING, BUT NOT LIMITED TO,
  THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
  PURPOSE.  THE ENTIRE RISK AS TO THE QUALITY AND PERFORMANCE OF THE PROGRAM
  IS WITH YOU.  SHOULD THE PROGRAM PROVE DEFECTIVE, YOU ASSUME THE COST OF
  ALL NECESSARY SERVICING, REPAIR OR CORRECTION.\n");
}

fn do_query(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                _ => return Err("Invalid mode".to_string())
            }
            info!("Querying info for {{{:?}}}", q);
            let mut queried_path = env.builds_dir.clone().unwrap();
            let tagdir_name = format!("v{}", q);
            queried_path.push(tagdir_name);

            if queried_path.exists() {
                trace!("Found {{{}}}", queried_path.display());
                queried_path.push(env.bin.clone().unwrap());
                if queried_path.exists() {
                    trace!("Found {{{}}}", queried_path.display());
                    if queried_path.is_file() {
                        info!("{} is a file", queried_path.display());
                        return Ok("Is a file".to_string());
                    } else {
                        debug!("{} is not a file", queried_path.display());
                        return Err("Not a file".to_string())
                    }
                } else {
                    warn!("No file found for {{{}}}", queried_path.display());
                    return Err("No file found".to_string())
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                return Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided for query op.");
            return Err("No tag provided.".to_string())
        }
    }
}

fn do_build(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref query) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(query) {
                        error!("{{{}}} was not a valid tag.",query);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(query) {
                        error!("{{{}}} was not a valid tag.",query);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::TestMode => {
                    todo!("Build op for test mode");
                }
                AmbosoMode::TestMacro => {
                    todo!("Build op for test macro");
                }
            }
            info!("Trying to build {{{:?}}}", query);
            let mut queried_path = env.builds_dir.clone().unwrap();
            let tagdir_name = format!("v{}", query);
            queried_path.push(tagdir_name);

            if queried_path.exists() {
                trace!("Found {{{}}}", queried_path.display());
                queried_path.push(env.bin.clone().unwrap());
                if queried_path.exists() {
                    trace!("Found {{{}}}", queried_path.display());
                    if queried_path.is_file() {
                        trace!("{} is a file, overriding it", queried_path.display());
                    } else {
                        error!("{} is not a file", queried_path.display());
                        return Err("Not a file".to_string())
                    }
                } else {
                    trace!("No file found for {{{}}}", queried_path.display());
                }

                let use_make = query >= &env.mintag_make.clone().unwrap();

                if use_make && !env.support_makemode {
                    error!("Can't build {{{}}}, as makemode is not supported by the project", query);
                    return Err("Missing makemode support".to_string());
                }

                let use_automake = query >= &env.mintag_automake.clone().unwrap();

                if use_automake && !env.support_automakemode {
                    error!("Can't build {{{}}}, as automakemode is not supported by the project", query);
                    return Err("Missing automakemode support".to_string());
                } else if use_automake {
                    match env.run_mode.as_ref().unwrap() {
                        AmbosoMode::GitMode => {
                            if cfg!(target_os = "windows") {
                                todo!("Support windows automake prep?");
                                /*
                                 * let output = Command::new("cmd")
                                 *   .args(["/C", "echo hello"])
                                 *   .output()
                                 *   .expect("failed to execute process")
                                 */
                            } else {
                                let output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("aclocal ; autoconf ; automake --add-missing ; ./configure"))
                                    .output()
                                    .expect("failed to execute process");

                                match output.status.code() {
                                    Some(autotools_prep_ec) => {
                                        if autotools_prep_ec == 0 {
                                            debug!("Automake prep succeded with status: {}", autotools_prep_ec.to_string());
                                        } else {
                                            error!("Automake failed with status: {}", autotools_prep_ec.to_string());
                                            io::stdout().write_all(&output.stdout).unwrap();
                                            io::stderr().write_all(&output.stderr).unwrap();
                                            return Err("Automake prep failed".to_string());
                                        }
                                    }
                                    None => {
                                        error!("Automake prep command failed");
                                        io::stdout().write_all(&output.stdout).unwrap();
                                        io::stderr().write_all(&output.stderr).unwrap();
                                        return Err("Automake prep command failed".to_string());
                                    }
                                }
                            };
                        }
                        _ => {
                            todo!("automake prep for {:?}", env.run_mode.as_ref().unwrap());
                        }
                    }
                }

                let output = if cfg!(target_os = "windows") {
                    todo!("Support windows build");
                    /*
                     * Command::new("cmd")
                     *   .args(["/C", "echo hello"])
                     *   .output()
                     *   .expect("failed to execute process")
                     */
                } else {
                    match env.run_mode.as_ref().unwrap() {
                        AmbosoMode::BaseMode => {
                            let build_path = PathBuf::from(format!("./{}/v{}/",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap()));
                            let mut source_path = build_path.clone();
                            source_path.push(env.source.clone().unwrap());
                            let mut bin_path = build_path.clone();
                            bin_path.push(env.bin.clone().unwrap());
                            if use_make {
                                trace!("Using make mode");
                                Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("( cd {} || echo \"cd failed\"; make )", build_path.display()))
                                    .output()
                                    .expect("failed to execute process")
                            } else {
                                Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("gcc {} -o {} -lm", source_path.display(), bin_path.display()))
                                    .output()
                                    .expect("failed to execute process")
                            }
                        }
                        AmbosoMode::GitMode => {
                            let build_path = PathBuf::from(format!("./{}/v{}/",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap()));
                            let mut source_path = build_path.clone();
                            source_path.push(env.source.clone().unwrap());
                            let mut bin_path = build_path.clone();
                            bin_path.push(env.bin.clone().unwrap());
                            trace!("Git mode, checking out {}",query);

                            let output = Command::new("sh")
                                .arg("-c")
                                .arg(format!("git checkout {} 2>/dev/null", query))
                                .output()
                                .expect("failed to execute process");

                            match output.status.code() {
                                Some(checkout_ec) => {
                                    if checkout_ec == 0 {
                                        debug!("Checkout succeded with status: {}", checkout_ec.to_string());
                                        let output = Command::new("sh")
                                            .arg("-c")
                                            .arg(format!("git submodule update --init --recursive"))
                                            .output()
                                            .expect("failed to execute process");
                                        match output.status.code() {
                                            Some(gsinit_ec) => {
                                                if gsinit_ec == 0 {
                                                    debug!("Submodule init succeded with status: {}", gsinit_ec.to_string());
                                                    let output = Command::new("sh")
                                                        .arg("-c")
                                                        .arg(format!("make >&2"))
                                                        .output()
                                                        .expect("failed to execute process");
                                                    match output.status.code() {
                                                        Some(make_ec) => {
                                                            if make_ec == 0 {
                                                               debug!("make succeded with status: {}", make_ec.to_string());
                                                                let output = Command::new("sh")
                                                                    .arg("-c")
                                                                    .arg(format!("mv {} {}", env.bin.as_ref().unwrap(), bin_path.display()))
                                                                    .output()
                                                                    .expect("failed to execute process");
                                                                match output.status.code() {
                                                                    Some(mv_ec) => {
                                                                        if mv_ec == 0 {
                                                                            debug!("mv succeded with status: {}", mv_ec.to_string());
                                                                            let output = Command::new("sh")
                                                                                .arg("-c")
                                                                                .arg(format!("git switch -"))
                                                                                .output()
                                                                                .expect("failed to execute process");
                                                                            match output.status.code() {
                                                                                Some(gswitch_ec) => {
                                                                                    if gswitch_ec == 0 {
                                                                                       debug!("git switch succeded with status: {}", gswitch_ec.to_string());
                                                                                        let output = Command::new("sh")
                                                                                            .arg("-c")
                                                                                            .arg(format!("git submodule update --init --recursive"))
                                                                                            .output()
                                                                                            .expect("failed to execute process");
                                                                                        match output.status.code() {
                                                                                            Some(gsinit_end_ec) => {
                                                                                                if gsinit_end_ec == 0 {
                                                                                                    debug!("git submodule init succeded with status: {}", gsinit_end_ec.to_string());
                                                                                                    debug!("Done build for {}", query);
                                                                                                } else {
                                                                                                    warn!("git submodule init failed with status: {}", gsinit_end_ec.to_string());
                                                                                                    io::stdout().write_all(&output.stdout).unwrap();
                                                                                                    io::stderr().write_all(&output.stderr).unwrap();
                                                                                                    return Err("git submodule init failed".to_string());
                                                                                                }
                                                                                            }
                                                                                            None => {
                                                                                                error!("git submodule init command failed");
                                                                                                io::stdout().write_all(&output.stdout).unwrap();
                                                                                                io::stderr().write_all(&output.stderr).unwrap();
                                                                                                return Err("git submodule init command failed".to_string());
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        warn!("git switch failed with status: {}", gswitch_ec.to_string());
                                                                                        io::stdout().write_all(&output.stdout).unwrap();
                                                                                        io::stderr().write_all(&output.stderr).unwrap();
                                                                                        return Err("git switch failed".to_string());
                                                                                    }
                                                                                }
                                                                                None => {
                                                                                    error!("git switch command failed");
                                                                                    io::stdout().write_all(&output.stdout).unwrap();
                                                                                    io::stderr().write_all(&output.stderr).unwrap();
                                                                                    return Err("git switch command failed".to_string());
                                                                                }
                                                                            }
                                                                        } else {
                                                                            warn!("mv failed with status: {}", mv_ec.to_string());
                                                                            io::stdout().write_all(&output.stdout).unwrap();
                                                                            io::stderr().write_all(&output.stderr).unwrap();
                                                                            return Err("mv failed".to_string());
                                                                        }
                                                                    }
                                                                    None => {
                                                                        error!("mv command failed");
                                                                        io::stdout().write_all(&output.stdout).unwrap();
                                                                        io::stderr().write_all(&output.stderr).unwrap();
                                                                        return Err("mv command failed".to_string());
                                                                    }
                                                                }
                                                            } else {
                                                                warn!("make failed with status: {}", make_ec.to_string());
                                                                io::stdout().write_all(&output.stdout).unwrap();
                                                                io::stderr().write_all(&output.stderr).unwrap();
                                                                return Err("make failed".to_string());
                                                            }
                                                        }
                                                        None => {
                                                            error!("make command failed");
                                                            io::stdout().write_all(&output.stdout).unwrap();
                                                            io::stderr().write_all(&output.stderr).unwrap();
                                                            return Err("make command failed".to_string());
                                                        }
                                                    }
                                                } else {
                                                    warn!("Submodule init failed with status: {}", gsinit_ec.to_string());
                                                    io::stdout().write_all(&output.stdout).unwrap();
                                                    io::stderr().write_all(&output.stderr).unwrap();
                                                    return Err("Submodule init failed".to_string());
                                                }
                                            }
                                            None => {
                                                error!("git submodule init command failed");
                                                io::stdout().write_all(&output.stdout).unwrap();
                                                io::stderr().write_all(&output.stderr).unwrap();
                                                return Err("git submodule init command failed".to_string());
                                            }
                                        }
                                    } else {
                                        warn!("Checkout failed with status: {}", checkout_ec.to_string());
                                        io::stdout().write_all(&output.stdout).unwrap();
                                        io::stderr().write_all(&output.stderr).unwrap();
                                        return Err("Checkout failed".to_string());
                                    }
                                    io::stdout().write_all(&output.stdout).unwrap();
                                    io::stderr().write_all(&output.stderr).unwrap();
                                    return Ok("Build done".to_string());
                                }
                                None => {
                                    error!("Git checkout command failed");
                                    io::stdout().write_all(&output.stdout).unwrap();
                                    io::stderr().write_all(&output.stderr).unwrap();
                                    return Err("Git checkout command failed".to_string());
                                }
                            }
                        }
                        _ => {
                            todo!("Build op for test modes");
                        }
                    }
                };
                match output.status.code() {
                    Some(x) => {
                        if x == 0 {
                            info!("Build succeded with status: {}", x.to_string());
                        } else {
                            warn!("Build failed with status: {}", x.to_string());
                        }
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Ok("Build done".to_string());
                    }
                    None => {
                        error!("Build command failed");
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Err("Build command failed".to_string());
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                return Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            return Err("No tag provided".to_string())
        }
    }
}

fn do_run(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::TestMode => {
                    todo!("Run op for test mode");
                }
                AmbosoMode::TestMacro => {
                    todo!("Run op for test macro");
                }
            }
            info!("Trying to run {{{:?}}}", q);
            let mut queried_path = env.builds_dir.clone().unwrap();
            let tagdir_name = format!("v{}", q);
            queried_path.push(tagdir_name);

            if queried_path.exists() {
                trace!("Found {{{}}}", queried_path.display());
                queried_path.push(env.bin.clone().unwrap());
                if queried_path.exists() {
                    trace!("Found {{{}}}", queried_path.display());
                    if queried_path.is_file() {
                        trace!("{} is a file", queried_path.display());
                    } else {
                        error!("{} is not a file", queried_path.display());
                        return Err("Not a file".to_string())
                    }
                } else {
                    warn!("No file found for {{{}}}", queried_path.display());
                    if ! env.do_build {
                        warn!("Try running with -b to build");
                    }
                    return Err("File not found".to_string());
                }

                let output = if cfg!(target_os = "windows") {
                    todo!("Support windows run");
                    /*
                     * Command::new("cmd")
                     *   .args(["/C", "echo hello"])
                     *   .output()
                     *   .expect("failed to execute process")
                     */
                } else {
                    let bin_path = PathBuf::from(format!("./{}/v{}/{}",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap(), env.bin.clone().unwrap()));
                    Command::new("sh")
                    .arg("-c")
                    .arg(format!("{}", bin_path.display()))
                    .output()
                    .expect("failed to execute process")
                };
                match output.status.code() {
                    Some(x) => {
                        if x == 0 {
                            info!("Run succeded with status: {}", x.to_string());
                        } else {
                            warn!("Run failed with status: {}", x.to_string());
                        }
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Ok("Run done".to_string());
                    }
                    None => {
                        error!("Run command for {{{}}} failed", args.tag.as_ref().unwrap());
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Err("Run command failed".to_string());
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                return Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            return Err("No tag provided".to_string())
        }
    }
}

fn do_delete(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(q) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::TestMode => {
                    todo!("Delete op for test mode");
                }
                AmbosoMode::TestMacro => {
                    todo!("Delete op for test macro");
                }
            }
            info!("Trying to delete {{{:?}}}", q);
            let mut queried_path = env.builds_dir.clone().unwrap();
            let tagdir_name = format!("v{}", q);
            queried_path.push(tagdir_name);

            if queried_path.exists() {
                trace!("Found {{{}}}", queried_path.display());
                queried_path.push(env.bin.clone().unwrap());
                if queried_path.exists() {
                    trace!("Found {{{}}}", queried_path.display());
                    if queried_path.is_file() {
                        trace!("{} is a file", queried_path.display());
                    } else {
                        error!("{} is not a file", queried_path.display());
                        return Err("Not a file".to_string())
                    }
                } else {
                    warn!("No file found for {{{}}}", queried_path.display());
                    return Err("File not found".to_string());
                }

                let output = if cfg!(target_os = "windows") {
                    todo!("Support windows delete");
                    /*
                     * Command::new("cmd")
                     *   .args(["/C", "echo hello"])
                     *   .output()
                     *   .expect("failed to execute process")
                     */
                } else {
                    let bin_path = PathBuf::from(format!("./{}/v{}/{}",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap(), env.bin.clone().unwrap()));
                    Command::new("sh")
                    .arg("-c")
                    .arg(format!("rm -f {}", bin_path.display()))
                    .output()
                    .expect("failed to execute process")
                };
                match output.status.code() {
                    Some(x) => {
                        if x == 0 {
                            info!("Delete succeded with status: {}", x.to_string());
                        } else {
                            warn!("Delete failed with status: {}", x.to_string());
                        }
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Ok("Delete done".to_string());
                    }
                    None => {
                        error!("Delete command for {{{}}} failed", args.tag.as_ref().unwrap());
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        return Err("Delete command failed".to_string());
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                return Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            return Err("No tag provided".to_string())
        }
    }
}

fn handle_amboso_env(env: AmbosoEnv, args: Args) {
    match env.run_mode {
        Some(ref runmode) => {
            info!("Runmode: {:?}", runmode);
            match runmode {
                    AmbosoMode::TestMode | AmbosoMode::TestMacro => {
                        if !env.support_testmode {
                            error!("Test mode not supported for this project.");
                            return
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

fn main() -> ExitCode {

    let mut args: Args = Args::parse();

    let log_level;

    if args.warranty {
        print_warranty_info();
    }

    if args.version {
        println!("{}",INVIL_VERSION);
        return ExitCode::SUCCESS;
    }

    if args.quiet && args.verbose >0 {
        args.verbose -= 1;
    }

    match args.verbose {
        5 => {
            log_level = LevelFilter::Trace;
        },
        4 => {
            log_level = LevelFilter::Debug;
        },
        3 => {
            log_level = LevelFilter::Info;
        },
        2 => {
            log_level = LevelFilter::Warn;
        },
        1|0 => {
            log_level = LevelFilter::Error;
        },
        _ => {
            log_level = LevelFilter::Debug;
        },
    }

    let config = ConfigBuilder::new()
        .set_level_color(Level::Error, Some(Color::Red))
        .set_level_color(Level::Trace, Some(Color::White))
        .set_level_color(Level::Warn, Some(Color::Yellow))
        .set_level_color(Level::Debug, Some(Color::Magenta))
        .set_level_color(Level::Info, Some(Color::Green))
        .set_time_level(LevelFilter::Debug)
        .set_thread_level(LevelFilter::Trace)
        .set_thread_mode(ThreadLogMode::Both)
        .build();

    match args.logged {
        false => {
            CombinedLogger::init(
                vec![
                    TermLogger::new(log_level, config, TerminalMode::Mixed, ColorChoice::Always),
                ]
            ).unwrap();
        }
        true => {
            CombinedLogger::init(
                vec![
                TermLogger::new(log_level, config.clone(), TerminalMode::Mixed, ColorChoice::Always),
                WriteLogger::new(LevelFilter::Trace, config, File::create(INVIL_LOG_FILE).unwrap()),
                ]
            ).unwrap();
        }
    }

    //Debug pretty-print of args
    trace!("Args: {:#?}\n", args);
    trace!("Log level: {}\n", log_level);

    if ! prog_name().expect("Failed resolvig current program name").eq("anvil") {
        trace!("Please symlink me to anvil.");
    }

    let invil_splash: String = format!("{}, version {}\nCopyright (C) 2023  jgabaut\n\n  This program comes with ABSOLUTELY NO WARRANTY; for details type `{} -W`.\n  This is free software, and you are welcome to redistribute it\n  under certain conditions; see file `LICENSE` for details.\n\n  Full source is available at https://github.com/jgabaut/invil\n", INVIL_NAME, INVIL_VERSION, prog_name().expect("Could not determine program name"));
    if ! args.quiet {
        println!("{}", invil_splash);
    }
    let res_check = check_passed_args(&mut args);

    match res_check {
        Ok(env) => {
            trace!("check_passed_args() success");
            match env.run_mode {
                Some(_) => {
                    handle_amboso_env(env, args);
                    return ExitCode::SUCCESS;
                }
                None => {
                    return ExitCode::SUCCESS;
                }
            }
        }
        Err(e) => {
            error!("check_passed_args() failed with: \"{}\"",e);
            return ExitCode::FAILURE;
        }
    }
}

