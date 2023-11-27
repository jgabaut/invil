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

const INVIL_VERSION: &str = env!("CARGO_PKG_VERSION");
const INVIL_NAME: &str = env!("CARGO_PKG_NAME");


#[derive(Parser, Debug)]
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
    #[arg(short = 'i', long, default_value = "false", conflicts_with_all(["purge", "delete", "build", "run"]))]
    init: bool,

    /// Delete binaries for all tags for current mode
    #[arg(short = 'p', long, default_value = "false", conflicts_with_all(["init", "delete", "build", "run"]))]
    purge: bool,

    /// Delete binary for passed tag
    #[arg(short = 'd', long, default_value = "false", conflicts_with_all(["init", "purge", "build", "run"]))]
    delete: bool,

    /// Build binary for passed tag
    #[arg(short = 'b', long, default_value = "false", conflicts_with_all(["init", "purge", "delete"]))]
    build: bool,

    /// Run binary for passed tag
    #[arg(short = 'r', long, default_value = "false", conflicts_with_all(["init", "purge", "delete"]))]
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
    #[arg(short = 'V', long, default_value = "0", conflicts_with_all(["quiet", "silent"]))]
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

    //TODO: Handle -C flag for passing start time for recursive calls

    /// Subcommand
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
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
            info!("Passed amboso_dir: {{{}}}", x.display());
            config_string.push_str(&amboso_dir_string);
        }
        None => {}
    }
    match args.kazoj_dir {
        Some(ref x) => {
            info!("Passed kazoj_dir: {{{}}}", x.display());
            config_string.push_str(&kazoj_dir_string);
        }
        None => {}
    }
    match args.source {
        Some(ref x) => {
            info!("Passed source: {{{}}}", x);
            config_string.push_str(&source_string);
        }
        None => {}
    }
    match args.execname {
        Some(ref x) => {
            info!("Passed execname: {{{}}}", x);
            config_string.push_str(&execname_string);
        }
        None => {}
    }
    match args.maketag {
        Some(ref x) => {
            info!("Passed maketag: {{{}}}", x);
            config_string.push_str(&maketag_string);
        }
        None => {}
    }
    if args.ignore_gitcheck {
        info!("Ignore git check is on.");
        config_string.push_str(&ignore_gitcheck_string);
    }
    info!("Config flags: {{-{}}}", config_string);
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
    info!("Mode flags: {{-{}}}", flags_string);
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
                info!("Passed dir to init: {}", init_dir.as_ref().expect("Missing init_dir").display());
            } else {
                warn!("Missing init_dir arg for init command.");
                //init_dir = &Some(PathBuf::from("."));
                //info!("Set . as init_dir");
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

    info!("Info flags: {{-{}}}", info_flags_string);
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

    info!("Op flags: {{-{}}}", op_flags_string);
}


fn print_grouped_args(args: &Args) {
    // Log asserted flags
    print_subcommand_args(&args);
    print_config_args(&args);
    print_mode_args(&args);
    print_info_args(&args);
    print_op_args(&args);
}

fn check_amboso_dir(dir: &PathBuf) -> bool {
    if dir.exists() {
        info!("Found {}", dir.display());
        let mut stego_path = dir.clone();
        stego_path.push("stego.lock");
        if stego_path.exists() {
            info!("Found {}", stego_path.display());
            let stego_contents = fs::read_to_string(stego_path).expect("Could not read stego contents");
            debug!("Stego contents: {{{}}}", stego_contents);
            todo!("Parsing stego info");
        } else {
            error!("Can't find {}. Quitting", stego_path.display());
            return false
        }
    } else {
        error!("Can't find {}. Quitting", dir.display());
        return false
    }
}

fn check_passed_args(args: &mut Args) {

    if args.warranty {
        print_warranty_info();
    }
    if args.version {
        println!("{}",INVIL_VERSION);
        return
    }

    print_grouped_args(&args);

    //Process arguments
    if args.ignore_gitcheck {
        info!("Ignoring git check.");
    } else {
        todo!("Gitcheck function");
    }
    match args.amboso_dir {
        Some(ref x) => {
            info!("Amboso dir {{{}}}", x.display());
            let res = check_amboso_dir(x);
            if res {
                debug!("Check pass: amboso_dir");
            } else {
                error!("Check fail: amboso_dir");
                return
            }
            debug!("TODO:    Validate amboso_dir and check its contained stego.lock");
        }
        None => {
            error!("Missing amboso dir argument. Quitting.");
            return
        }
    }
    match args.kazoj_dir {
        Some(ref x) => {
            info!("Tests dir {{{}}}", x.display());
            debug!("TODO:    Validate kazoj_dir");
        }
        None => {
            warn!("Missing tests dir.");
            args.kazoj_dir = Some(PathBuf::from("./kazoj"));
            info!("Set default tests dir: {{{}}}.",
                    args.kazoj_dir.as_ref()
                    .expect("./kazoj was not a valid path").display());
            debug!("TODO:    Validate kazoj_dir");
        }
    }

    match args.source {
        Some(ref x) => {
            info!("Source {{{}}}", x);
            debug!("TODO:  Validate source")
        }
        None => {
            warn!("Missing source arg.");
            debug!("TODO:    Get source arg from stego.lock");
        }
    }

    match args.execname {
        Some(ref x) => {
            info!("Execname {{{}}}", x);
            debug!("TODO:  Validate execname")
        }
        None => {
            warn!("Missing execname arg.");
            debug!("TODO:    Get execname arg from stego.lock");
        }
    }

    match args.maketag {
        Some(ref x) => {
            info!("Maketag {{{}}}", x);
            debug!("TODO:  Validate maketag")
        }
        None => {
            warn!("Missing maketag arg.");
            debug!("TODO:    Get maketag arg from stego.lock");
        }
    }

    match args.gen_c_header {
        Some(ref x) => {
            info!("C header dir: {{{}}}", x.display());
            debug!("TODO:    Validate C header dir");
        }
        None => {
            trace!("-G not asserted.");
        }
    }

    match args.linter {
        Some(ref x) => {
            info!("Linter for file: {{{}}}", x.display());
            debug!("TODO:    Validate linter file arg");
        }
        None => {
            trace!("-x not asserted.");
        }
    }

    todo!("Check all required arguments are usable, and if they aren't either set them or fail");
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

fn main() {

    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            //WriteLogger::new(LevelFilter::Info, Config::default(), File::create("my_rust_binary.log").unwrap()),
        ]
    ).unwrap();

    let mut args: Args = Args::parse();

    //Debug pretty-print of args
    trace!("Args: {:#?}\n", args);

    if ! prog_name().expect("Failed resolvig current program name").eq("anvil") {
        trace!("Please symlink me to anvil.");
    }

    let invil_splash: String = format!("{}, version {}\nCopyright (C) 2023  jgabaut\n\n  This program comes with ABSOLUTELY NO WARRANTY; for details type `{} -W`.\n  This is free software, and you are welcome to redistribute it\n  under certain conditions; see file `LICENSE` for details.\n\n  Full source is available at https://github.com/jgabaut/invil\n", INVIL_NAME, INVIL_VERSION, prog_name().expect("Could not determine program name"));
    println!("{}", invil_splash);
    check_passed_args(&mut args);
}

