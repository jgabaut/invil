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
use crate::core::{Args, Commands};
use std::{env};
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::fs;
use toml::Table;

pub fn prog_name() -> Option<String> {
    env::current_exe().ok()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
}

pub fn print_config_args(args: &Args) {
    //Handle config flags
    let mut config_string: String = "".to_owned();
    let amboso_dir_string: String = "D".to_owned();
    let kazoj_dir_string: String = "K".to_owned();
    let source_string: String = "S".to_owned();
    let execname_string: String = "E".to_owned();
    let maketag_string: String = "M".to_owned();
    let ignore_gitcheck_string: String = "X".to_owned();
    let anvil_version_string: String = "a".to_owned();
    let anvil_kern_string: String = "k".to_owned();
    if let Some(ref x) = args.amboso_dir {
        debug!("Passed amboso_dir: {{{}}}", x.display());
        config_string.push_str(&amboso_dir_string);
    }
    if let Some(ref x) = args.kazoj_dir {
        debug!("Passed kazoj_dir: {{{}}}", x.display());
        config_string.push_str(&kazoj_dir_string);
    }
    if let Some(ref x) = args.source {
        debug!("Passed source: {{{}}}", x);
        config_string.push_str(&source_string);
    }
    if let Some(ref x) = args.execname {
        debug!("Passed execname: {{{}}}", x);
        config_string.push_str(&execname_string);
    }
    if let Some(ref x) = args.maketag {
        debug!("Passed maketag: {{{}}}", x);
        config_string.push_str(&maketag_string);
    }
    if let Some(ref x) = args.anvil_version {
        debug!("Passed anvil_version: {{{}}}", x);
        config_string.push_str(&anvil_version_string);
    }
    match args.anvil_kern {
        Some(ref x) => {
            debug!("Passed anvil_kern: {{{}}}", x);
            config_string.push_str(&anvil_kern_string);
        }
        None => {
            trace!("No anvil_kern value");
            panic!("Missing anvil_kern value");
        }
    }
    if args.ignore_gitcheck {
        debug!("Ignore git check is on.");
        config_string.push_str(&ignore_gitcheck_string);
    }
    debug!("Config flags: {{-{}}}", config_string);
}

pub fn print_mode_args(args: &Args) {
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
    if args.gen_c_header.is_some() {
        flags_string.push_str(&gen_c_mode_string);
    }
    if args.linter.is_some() {
        flags_string.push_str(&linter_mode_string);
    }
    debug!("Mode flags: {{-{}}}", flags_string);
}

pub fn print_subcommand_args(args: &Args) {
    match &args.command {
        Some(Commands::Test { list, query: _, build: _}) => {
            if *list {
                debug!("Printing testing lists...");
            } else {
                debug!("Not printing testing lists...");
            }
        }
        Some(Commands::Build) => {
            debug!("Doing quick build command")
        }
        Some(Commands::Init { kern, init_dir, template_name }) => {
            if kern.is_some() {
                debug!("Passed kern: {}", kern.as_ref().expect("Missing kern"));
            }
            if init_dir.is_some() {
                debug!("Passed dir to init: {}", init_dir.as_ref().expect("Missing init_dir").display());
            } else {
                warn!("Missing init_dir arg for init command.");
            }
            if kern.is_some() && kern.clone().expect("Missing kern").as_str() == "custom" {
                if template_name.is_some() {
                    debug!("Passed template_name to init: {}", template_name.clone().expect("Missing template_name"));
                } else {
                    warn!("Missing template_name arg for init command.");
                }
            }
        }
        Some(Commands::Version) => {
            debug!("Printing version");
        }
        None => {}
    }
}

pub fn print_info_args(args: &Args) {
    //Handle info flags
    let mut info_flags_string: String = "".to_owned();

    if args.version {
        info_flags_string.push('v');
    }
    if args.watch {
        info_flags_string.push('w');
    }
    if args.quiet {
        info_flags_string.push('q');
    }
    if args.silent {
        info_flags_string.push('s');
    }
    if args.list {
        info_flags_string.push('l');
    }
    if args.list_all {
        info_flags_string.push('L');
    }
    if args.warranty {
        info_flags_string.push('W');
    }

    debug!("Info flags: {{-{}}}", info_flags_string);
}

pub fn print_op_args(args: &Args) {
    //Handle op flags
    let mut op_flags_string: String = "".to_owned();

    if args.build {
        op_flags_string.push('b');
    }
    if args.run {
        op_flags_string.push('r');
    }
    if args.delete {
        op_flags_string.push('d');
    }
    if args.init {
        op_flags_string.push('i');
    }
    if args.purge {
        op_flags_string.push('p');
    }

    debug!("Op flags: {{-{}}}", op_flags_string);
}

pub fn print_extension_args(args: &Args) {
    //Handle mode flags
    let mut flags_string: String = "".to_owned();
    let no_rebuild_string: String = "R".to_owned();
    let force_rebuild_string: String = "F".to_owned();
    let logged_string: String = "J".to_owned();
    let no_color_string: String = "P".to_owned();
    if args.no_rebuild {
        flags_string.push_str(&no_rebuild_string);
    }
    if args.force {
        flags_string.push_str(&force_rebuild_string);
    }
    if args.logged {
        flags_string.push_str(&logged_string);
    }
    if args.no_color {
        flags_string.push_str(&no_color_string);
    }
    debug!("Extension flags: {{-{}}}", flags_string);
}

fn print_strict_mode_notice(args: &Args) {
    if args.strict {
        warn!("Running in strict mode.");
    } else {
        debug!("No --strict, running with extensions.");
    }
}


pub fn print_grouped_args(args: &Args) {
    // Log asserted flags
    print_subcommand_args(args);
    print_config_args(args);
    print_mode_args(args);
    print_info_args(args);
    print_op_args(args);
    print_extension_args(args);
    print_strict_mode_notice(args);
}

pub fn print_warranty_info() {
    println!("  THERE IS NO WARRANTY FOR THE PROGRAM, TO THE EXTENT PERMITTED BY
  APPLICABLE LAW.  EXCEPT WHEN OTHERWISE STATED IN WRITING THE COPYRIGHT
  HOLDERS AND/OR OTHER PARTIES PROVIDE THE PROGRAM \"AS IS\" WITHOUT WARRANTY
  OF ANY KIND, EITHER EXPRESSED OR IMPLIED, INCLUDING, BUT NOT LIMITED TO,
  THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
  PURPOSE.  THE ENTIRE RISK AS TO THE QUALITY AND PERFORMANCE OF THE PROGRAM
  IS WITH YOU.  SHOULD THE PROGRAM PROVE DEFECTIVE, YOU ASSUME THE COST OF
  ALL NECESSARY SERVICING, REPAIR OR CORRECTION.\n");
}

pub fn try_parse_stego(stego_path: &PathBuf) -> Result<String,String> {
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    trace!("Stego contents: {{{}}}", stego);
    let toml_value = stego.parse::<Table>();
    match toml_value {
        Ok(_) => {
            debug!("try_parse_stego(): Lint success for {{{}}}", stego_path.display());
            Ok("Lint success".to_string())
        }
        Err(e) => {
            error!("Lint failed for {{{}}}. Err: {e}", stego_path.display());
            Err("Lint failed".to_string())
        }
    }
}
