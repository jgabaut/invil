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

mod core;
mod ops;
mod utils;
#[cfg(feature = "anvilPy")]
mod anvil_py;
#[cfg(feature = "anvilCustom")]
mod anvil_custom;

#[macro_use] extern crate log;
use simplelog::*;
use std::process::{ExitCode, exit};
use std::fs::File;
use crate::core::{Args, Commands,
    INVIL_NAME,
    INVIL_VERSION,
    INVIL_LOG_FILE,
    check_passed_args,
    handle_amboso_env,
    handle_init_subcommand,
    AmbosoLintMode
};
use crate::utils::{
    print_warranty_info,
    prog_name,
};
use crate::ops::{
    handle_linter_flag,
};
use clap::Parser;

fn main() -> ExitCode {

    let mut args: Args = Args::parse();

    let log_level;

    if args.version {
        if args.verbose > 3 {
            println!("invil, v{} (Compat: v{})",INVIL_VERSION, args.anvil_version.expect("Failed initialising anvil_version"));
            if cfg!(feature = "anvilPy") {
                println!("Experimental anvilPy support is enabled.");
            } else {
                println!("Experimental anvilPy support is NOT enabled.");
            }
            if cfg!(feature = "anvilCustom") {
                println!("Experimental anvilCustom support is enabled.");
            } else {
                println!("Experimental anvilCustom support is NOT enabled.");
            }
        } else {
            println!("{}",INVIL_VERSION);
        }
        return ExitCode::SUCCESS;
    }

    if args.quiet && args.verbose >0 {
        args.verbose -= 1;
    }

    match args.silent {
        true => {
            log_level = LevelFilter::Error;
        }
        false => {
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

        }
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

    let color_choice = if args.no_color {
        ColorChoice::Never
    } else {
        ColorChoice::Always
    };

    match args.logged {
        false => {
            CombinedLogger::init(
                vec![
                    TermLogger::new(log_level, config, TerminalMode::Mixed, color_choice),
                ]
            ).unwrap();
        }
        true => {
            CombinedLogger::init(
                vec![
                TermLogger::new(log_level, config.clone(), TerminalMode::Mixed, color_choice),
                WriteLogger::new(LevelFilter::Trace, config, File::create(INVIL_LOG_FILE).unwrap()),
                ]
            ).unwrap();
        }
    }

    //Debug pretty-print of args
    //trace!("Args: {:#?}\n", args);
    trace!("Log level: {}\n", log_level);

    if ! prog_name().expect("Failed resolvig current program name").eq("anvil") {
        trace!("Please symlink me to anvil.");
    }

    let invil_splash: String = format!("{}, version {}\nCopyright (C) 2023-2025  jgabaut\n\n  This program comes with ABSOLUTELY NO WARRANTY; for details type `{} -W`.\n  This is free software, and you are welcome to redistribute it\n  under certain conditions; see file `LICENSE` for details.\n\n  Full source is available at https://github.com/jgabaut/invil\n", INVIL_NAME, INVIL_VERSION, prog_name().expect("Could not determine program name"));
    if ! args.quiet {
        println!("{}", invil_splash);
    }

    if args.warranty {
        print_warranty_info();
    }

    match args.command {
        Some(Commands::Init { init_dir }) => {
            return handle_init_subcommand(init_dir, args.strict);
        }
        Some(Commands::Version) => {
            println!("{INVIL_VERSION}\n");
            return ExitCode::SUCCESS;
        }
        _ => {} //Other subcommands may be handled later, in handle_amboso_env()
    }

    match args.linter {
        Some(ref x) => {
            let mut lint_mode = AmbosoLintMode::FullCheck;
            if args.list_all {
                lint_mode = AmbosoLintMode::Lex;
            }
            if args.list {
                lint_mode = AmbosoLintMode::LintOnly;
            }
            if args.ignore_gitcheck {
                lint_mode = match args.list_all {
                    true => {
                        AmbosoLintMode::NajloDebug
                    }
                    false => {
                        match args.quiet {
                            true => {
                                AmbosoLintMode::NajloQuiet
                            }
                            false => {
                                AmbosoLintMode::NajloFull
                            }
                        }
                    }
                }
            }
            let res = handle_linter_flag(x, &lint_mode);
            match res {
                Ok(s) => {
                    match lint_mode {
                        AmbosoLintMode::NajloFull => {
                            debug!("{s}");
                        }
                        _ => {
                            debug!("{s}");
                        }
                    }
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    error!("handle_linter_flag() failed. Err: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        None => {
            trace!("-x not asserted.");
        }
    }

    let res_check = check_passed_args(&mut args);

    match res_check {
        Ok(mut env) => {
            trace!("check_passed_args() success");
            let elapsed_checking_args = env.start_time.elapsed();
            if args.watch {
                info!("Done checking args. Elapsed: {:.2?}", elapsed_checking_args);
            }
            match env.run_mode {
                Some(_) => {
                    handle_amboso_env(&mut env, &mut args);
                    let elapsed_handling_args = env.start_time.elapsed();
                    if args.watch {
                        info!("Done handling args. Elapsed: {:.2?}", elapsed_handling_args);
                    }
                    ExitCode::SUCCESS
                }
                None => {
                    let elapsed_no_runmode = env.start_time.elapsed();
                    if args.watch {
                        info!("Done no runmode arg. Elapsed: {:.2?}", elapsed_no_runmode);
                    }
                    ExitCode::SUCCESS
                }
            }
        }
        Err(e) => {
            error!("check_passed_args() failed with: \"{}\"",e);
            ExitCode::FAILURE
        }
    }
}

