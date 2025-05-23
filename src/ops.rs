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
use crate::core::{Args, AmbosoEnv, AmbosoMode, AmbosoLintMode, AnvilKern, INVIL_VERSION, INVIL_OS, EXPECTED_AMBOSO_API_LEVEL, parse_stego_toml, lex_stego_toml, SemVerKey, ANVIL_INTERPRETER_TAG_REGEX, RULE_REGEX, RULELINE_MARK_CHAR, RULEWARN_REGEX, cut_line_at_char, CutDirection, semver_compare, MIN_AMBOSO_V_PYKERN, MIN_AMBOSO_V_CHECK_DETACHED};
use crate::utils::try_parse_stego;

use std::process::{Command, Stdio, exit};
use std::io::{self, Write, BufRead};
use std::path::{Path, PathBuf};
use is_executable::is_executable;
use std::collections::BTreeMap;
use std::fs::{self, File};
use git2::Repository;
use std::env;
use std::time::SystemTime;
use regex::Regex;
use std::cmp::Ordering;

#[cfg(feature = "anvilPy")]
use crate::anvil_py::{ANVILPY_UNPACKDIR_NAME,unpack_srcdist, post_unpack};


pub fn do_build(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref query) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(&SemVerKey(query.to_string())) {
                        error!("{{{}}} was not a valid tag.",query);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(&SemVerKey(query.to_string())) {
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

            trace!("Looking for {{{}}}", queried_path.display());
            trace!("Builds dir: {{{}}}", env.builds_dir.clone().unwrap().display());

            if ! queried_path.exists() {
                match fs::create_dir_all(queried_path.clone()) {
                    Ok(_) => {
                        debug!("Created query target dir, proceeding.");
                    }
                    Err(e) => {
                        error!("Failed creating query target dir. Err: {e}");
                        return Err("Failed creating query target dir.".to_string());
                    }
                }
            }

            if queried_path.exists() {
                trace!("Found {{{}}}", queried_path.display());
                queried_path.push(env.bin.clone().unwrap());
                if queried_path.exists() {
                    trace!("Found {{{}}}", queried_path.display());
                    if queried_path.is_file() {
                        trace!("{} is a file", queried_path.display());
                        if ! args.force {
                            info!("{{{}}} is ready at {{{}}}.", query, queried_path.display());
                            info!("Try running with --force to force build.");
                            return Ok("File was ready".to_string());
                        }
                    } else {
                        error!("{} is not a file", queried_path.display());
                        return Err("Not a file".to_string())
                    }
                } else {
                    trace!("No file found for {{{}}}", queried_path.display());
                }

                let mut use_make = false;
                if env.anvil_kern == AnvilKern::AmbosoC {
                    use_make = query >= &env.mintag_make.clone().unwrap();

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
                                    debug!("Running \'aclocal; autoconf; automake --add-missing;\'");
                                    let autoconf_bootstrap_cmd = "aclocal; autoconf; automake --add-missing;";
                                    let output = Command::new("sh")
                                        .arg("-c")
                                        .arg(autoconf_bootstrap_cmd)
                                        .output()
                                        .expect("failed to execute process");

                                    match output.status.code() {
                                        Some(autotools_bootstrap_ec) => {
                                            if autotools_bootstrap_ec == 0 {
                                                debug!("Automake bootstrap succeded with status: {}", autotools_bootstrap_ec.to_string());
                                            } else {
                                                error!("Automake bootstrap failed with status: {}", autotools_bootstrap_ec.to_string());
                                                io::stdout().write_all(&output.stdout).unwrap();
                                                io::stderr().write_all(&output.stderr).unwrap();
                                                return Err("Automake bootstrap failed".to_string());
                                            }
                                        }
                                        None => {
                                            error!("Automake prep command failed");
                                            io::stdout().write_all(&output.stdout).unwrap();
                                            io::stderr().write_all(&output.stderr).unwrap();
                                            return Err("Automake prep command failed".to_string());
                                        }
                                    }
                                    debug!("Running \'./configure \"{}\"\'", env.configure_arg);
                                    let autoconf_configure_cmd = "./configure";

                                    let output = Command::new(autoconf_configure_cmd)
                                        .arg(env.configure_arg.clone())
                                        .output()
                                        .expect("failed to execute process");

                                    match output.status.code() {
                                        Some(autotools_config_ec) => {
                                            if autotools_config_ec == 0 {
                                                debug!("Automake config succeded with status: {}", autotools_config_ec.to_string());
                                            } else {
                                                error!("Automake failed with status: {}", autotools_config_ec.to_string());
                                                io::stdout().write_all(&output.stdout).unwrap();
                                                io::stderr().write_all(&output.stderr).unwrap();
                                                return Err("Automake config failed".to_string());
                                            }
                                        }
                                        None => {
                                            error!("Automake config command failed");
                                            io::stdout().write_all(&output.stdout).unwrap();
                                            io::stderr().write_all(&output.stderr).unwrap();
                                            return Err("Automake config command failed".to_string());
                                        }
                                    }

                                };
                            }
                            _ => {
                                todo!("automake prep for {:?}", env.run_mode.as_ref().unwrap());
                            }
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
                            let head_was_detached = false; // We don't worry about HEAD if we're not in git
                                                           // mode
                            let build_path = PathBuf::from(format!("./{}/v{}/",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap()));
                            let mut source_path = build_path.clone();
                            source_path.push(env.source.clone().unwrap());
                            let mut bin_path = build_path.clone();
                            bin_path.push(env.bin.clone().unwrap());
                            let cflg_str;
                            if !env.cflags_arg.is_empty() { //We have the arg from --config/-Z
                                debug!("Using passed CFLAGS {{{}}}", env.cflags_arg);
                                cflg_str = env.cflags_arg.clone();
                            } else { //Backcomp reading env CFLAGS
                                let cflags = "CFLAGS";
                                match env::var(cflags) {
                                    Ok(val) => {
                                        debug!("Using {{{}: {}}}", cflags, val);
                                        cflg_str = val.to_string();
                                    },
                                    Err(e) => {
                                        error!("Failed reading {{{}: {}}}", cflags, e);
                                        cflg_str = "".to_string()
                                    }
                                }
                            }
                            let cc = "CC";
                            let cc_str = match env::var(cc) {
                                Ok(val) => {
                                    debug!("Using {{{}: {}}}", cc, val);
                                    val.to_string()
                                },
                                Err(e) => {
                                    error!("Failed reading {{{}: {}}}", cc, e);
                                    "gcc".to_string()
                                }
                            };
                            match env.anvil_kern {
                                AnvilKern::AmbosoC => {
                                    if use_make {
                                        trace!("Using make mode");
                                        let current_dir = env::current_dir().expect("failed getting current directory");
                                        let cd_output = env::set_current_dir(&build_path);
                                        if cd_output.is_ok() {
                                            match build_step(args, env, cflg_str, query, bin_path, build_path, env.bin.clone().unwrap(), head_was_detached) {
                                                Ok(s) => {
                                                    trace!("{s}");
                                                    let cdback_output = env::set_current_dir(&current_dir);
                                                    if cdback_output.is_ok() {
                                                        return Ok(s);
                                                    } else {
                                                        return Err(format!("cdback to {{{}}} failed", current_dir.display()));
                                                    }
                                                }
                                                Err(e) => {
                                                    return Err(format!("Build failed for {{{query}}}. Err: {e}"));
                                                }
                                            }
                                        } else {
                                            error!("cd to build_path {} failed", build_path.display());
                                            return Err("Failed cd to build_path".to_string());
                                        }
                                    } else {

                                        let single_mode_cmd = cc_str;
                                        trace!("Using single file mode: \'{} {} {} -o {} -lm\'", single_mode_cmd, cflg_str, source_path.display(), bin_path.display());
                                        Command::new(single_mode_cmd)
                                            .arg(cflg_str)
                                            .arg(source_path)
                                            .arg("-o")
                                            .arg(bin_path)
                                            .arg("-lm")
                                            .output()
                                            .expect("failed to execute process")
                                    }
                                }
                                AnvilKern::AnvilPy | AnvilKern::Custom => {
                                    let current_dir = env::current_dir().expect("failed getting current directory");
                                    let cd_output = env::set_current_dir(&build_path);
                                    if cd_output.is_ok() {
                                        match build_step(args, env, cflg_str, query, bin_path, build_path, env.bin.clone().unwrap(), head_was_detached) {
                                            Ok(s) => {
                                                trace!("{s}");
                                                let cdback_output = env::set_current_dir(&current_dir);
                                                if cdback_output.is_ok() {
                                                    return Ok(s);
                                                } else {
                                                    return Err(format!("cdback to {{{}}} failed", current_dir.display()));
                                                }
                                            }
                                            Err(e) => {
                                                return Err(format!("Build failed for {{{query}}}. Err: {e}"));
                                            }
                                        }
                                    } else {
                                        error!("cd to build_path {} failed", build_path.display());
                                        return Err("Failed cd to build_path".to_string());
                                    }
                                }
                            }
                        }
                        AmbosoMode::GitMode => {
                            let build_path = PathBuf::from(format!("{}/v{}/",env.builds_dir.as_ref().unwrap().display(), args.tag.as_ref().unwrap()));
                            let mut source_path = build_path.clone();
                            source_path.push(env.source.clone().unwrap());
                            let mut bin_path = build_path.clone();
                            bin_path.push(env.bin.clone().unwrap());
                            let cflg_str = if !env.cflags_arg.is_empty() {
                                debug!("Using passed CFLAGS{{{}}}", env.cflags_arg);
                                env.cflags_arg.clone()
                            } else {
                                "".to_string()
                            };

                            let repo = Repository::discover(".").expect("Failed to open repository");

                            let head_was_detached = match semver_compare(&env.anvil_version, MIN_AMBOSO_V_CHECK_DETACHED) {
                                Ordering::Less => {
                                    warn!("Strict behaviour for v{}, won't check if starting from detached HEAD", env.anvil_version);
                                    false
                                }
                                Ordering::Equal | Ordering::Greater => {
                                    match repo.head() {
                                        Ok(head) => {
                                            if head.is_branch() {
                                                //println!("On branch: {}", head.shorthand().unwrap_or("Unknown"));
                                                false
                                            } else {
                                                debug!("Starting from a detached HEAD");
                                                true
                                            }
                                        }
                                        Err(e) => {
                                            error!("Error retrieving HEAD: {}", e);
                                            false
                                        }
                                    }
                                }
                            };

                            trace!("Git mode, checking out {}",query);
                            trace!("Running \'git checkout {}\'", query);

                            let output = Command::new("git")
                                .arg("checkout")
                                .arg(query)
                                .stderr(Stdio::null())
                                .output()
                                .expect("failed to execute process");

                            match output.status.code() {
                                Some(checkout_ec) => {
                                    if checkout_ec == 0 {
                                        debug!("Checkout succeded with status: {}", checkout_ec.to_string());
                                        trace!("Running \'git submodule update --init --recursive\'");
                                        let output = Command::new("git")
                                            .arg("submodule")
                                            .arg("update")
                                            .arg("--init")
                                            .arg("--recursive")
                                            .output()
                                            .expect("failed to execute process");
                                        match output.status.code() {
                                            Some(gsinit_ec) => {
                                                if gsinit_ec == 0 {
                                                    debug!("Submodule init succeded with status: {}", gsinit_ec.to_string());
                                                    trace!("Build step");
                                                    trace!("cflg_str: {{{cflg_str}}}");
                                                    trace!("bin_path: {{{}}}", bin_path.display());
                                                    match build_step(args, env, cflg_str, query, bin_path, build_path, env.bin.clone().unwrap(), head_was_detached) {
                                                        Ok(s) => {
                                                            trace!("{s}");
                                                        }
                                                        Err(e) => {
                                                            return Err(format!("Build failed for {{{query}}}. Err: {e}"));
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
                        Ok("Build done".to_string())
                    }
                    None => {
                        error!("Build command failed");
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        Err("Build command failed".to_string())
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            Err("No tag provided".to_string())
        }
    }
}

pub fn do_run(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(&SemVerKey(q.to_string())) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(&SemVerKey(q.to_string())) {
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
                    trace!("Running \'{}\'", bin_path.display());
                    Command::new(bin_path)
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
                        Ok("Run done".to_string())
                    }
                    None => {
                        error!("Run command for {{{}}} failed", args.tag.as_ref().unwrap());
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        Err("Run command failed".to_string())
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            Err("No tag provided".to_string())
        }
    }
}

pub fn do_delete(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(&SemVerKey(q.to_string())) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(&SemVerKey(q.to_string())) {
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
                    trace!("Running \'rm -f {}\'", bin_path.display());
                    Command::new("rm")
                    .arg("-f")
                    .arg(bin_path)
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
                        Ok("Delete done".to_string())
                    }
                    None => {
                        error!("Delete command for {{{}}} failed", args.tag.as_ref().unwrap());
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        Err("Delete command failed".to_string())
                    }
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                Err("No dir found".to_string())
            }
        }
        None => {
            warn!("No tag provided.");
            Err("No tag provided".to_string())
        }
    }
}

pub fn do_query(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
    match args.tag {
        Some(ref q) => {
            let interpreter_regex = Regex::new(ANVIL_INTERPRETER_TAG_REGEX).expect("Failed to create ruleline regex");
            if interpreter_regex.is_match(q) {
                info!("Running as interpreter for {{{q}}}");
                handle_running_make();
            }
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::GitMode => {
                    if ! env.gitmode_versions_table.contains_key(&SemVerKey(q.to_string())) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::BaseMode => {
                    if ! env.basemode_versions_table.contains_key(&SemVerKey(q.to_string())) {
                        error!("{{{}}} was not a valid tag.",q);
                        return Err("Invalid tag".to_string())
                    }
                }
                AmbosoMode::TestMode => {
                    if ! env.support_testmode {
                        return Err("Missing testmode support".to_string());
                    } else {
                        let do_record = if env.do_build {
                            info!("Recording test {{{:?}}}", q);
                            true
                        } else {
                            info!("Testing {{{:?}}}", q);
                            false
                        };

                        let queried_path;
                        if ! env.bonetests_table.contains_key(q) && ! env.kulpotests_table.contains_key(q) {
                            error!("Not a valid test: {{{}}}", q);
                            return Err("Invalid test query".to_string());
                        } else if env.bonetests_table.contains_key(q) {
                            queried_path = env.bonetests_table.get(q);
                        } else {
                            queried_path = env.kulpotests_table.get(q);
                        }

                        match queried_path {
                            Some(qp) => {
                               if qp.exists() {
                                trace!("Found {{{}}}", qp.display());
                                if qp.is_file() {
                                    info!("{} is a file", qp.display());
                                    if is_executable(qp) {
                                        debug!("{} is executable", qp.display());
                                        let test_res = run_test(qp, do_record);

                                        return test_res;
                                    } else {
                                        debug!("{} is not executable", qp.display());
                                        return Ok("Is not executable".to_string());
                                    }
                                } else {
                                        debug!("{} is not a file", qp.display());
                                        return Err("Not a file".to_string())
                                }
                            } else {
                                warn!("No file found for {{{}}}", qp.display());
                                return Err("No file found".to_string())
                            }

                            }
                            None => {
                                error!("Not a valid test path.");
                                return Err("Invalid test query".to_string());
                            }
                        }
                    }
                }
                AmbosoMode::TestMacro => {
                    todo!("Support query in test macro");
                }
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
                        if is_executable(&queried_path) {
                            debug!("{} is executable", queried_path.display());
                            Ok("Is executable".to_string())
                        } else {
                            debug!("{} is not executable", queried_path.display());
                            Ok("Is not executable".to_string())
                        }

                    } else {
                        debug!("{} is not a file", queried_path.display());
                        Err("Not a file".to_string())
                    }
                } else {
                    warn!("No file found for {{{}}}", queried_path.display());
                    Err("No file found".to_string())
                }
            } else {
                warn!("No directory found for {{{}}}", queried_path.display());
                Err("No dir found".to_string())
            }
        }
        None => {
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::TestMacro => {
                    if ! env.support_testmode {
                        return Err("Missing testmode support".to_string());
                    } else {
                        let do_record = if env.do_build {
                            info!("Recording all tests");
                            true
                        } else {
                            info!("Running all tests");
                            false
                        };
                        let mut alltests_map: BTreeMap<String, PathBuf> = BTreeMap::new();
                        let mut bonetests_map = env.bonetests_table.clone();
                        let mut kulpotests_map = env.kulpotests_table.clone();
                        alltests_map.append(&mut bonetests_map);
                        alltests_map.append(&mut kulpotests_map);
                        let mut tot_successes = 0;
                        let mut tot_failures = 0;
                        for test in alltests_map.values() {
                            if test.exists() {
                                trace!("Found {{{}}}", test.display());
                                if test.is_file() {
                                    info!("{} is a file", test.display());
                                    if is_executable(test) {
                                        debug!("{} is executable", test.display());
                                        let test_res = run_test(test, do_record);

                                        if args.watch {
                                            let test_elapsed = env.start_time.elapsed();
                                            info!("Done test {{{}}}, Elapsed: {:.2?}", test.display(), test_elapsed);
                                        }
                                        match test_res {
                                            Ok(st) => {
                                                info!("Test ok: {st}");
                                                tot_successes += 1;
                                            }
                                            Err(e) => {
                                                error!("Test {} failed. Err: {e}", test.display());
                                                tot_failures += 1;
                                            }
                                        }
                                    } else {
                                        debug!("{} is not executable", test.display());
                                        return Ok("Is not executable".to_string());
                                    }
                                } else {
                                        debug!("{} is not a file", test.display());
                                        return Err("Not a file".to_string());
                                }
                            } else {
                                warn!("No file found for {{{}}}", test.display());
                                return Err("No file found".to_string())
                            }
                        }
                        debug!("Done test macro");
                        info!("Successes: {tot_successes}");
                        error!("Failures: {tot_failures}");
                        if tot_failures != 0 {
                            return Err("Test macro had some failures".to_string());
                        } else {
                            return Ok("Done test macro run".to_string());
                        }
                    }
                }
                AmbosoMode::GitMode | AmbosoMode::BaseMode => {
                    if ! env.do_init && ! env.do_purge && ! args.list && ! args.list_all {
                        handle_running_make();
                    }
                }
                _ => {}
            }
            warn!("No tag provided for query op.");
            Err("No tag provided.".to_string())
        }
    }
}

pub fn run_test(test_path: &PathBuf, record: bool) -> Result<String,String> {
    let output = if cfg!(target_os = "windows") {
        todo!("Support windows tests");
        /*
         * Command::new("cmd")
         *   .args(["/C", "echo hello"])
         *   .output()
         *   .expect("failed to execute process")
         */
    } else {
        trace!("Running \'{}\'", test_path.display());
        Command::new(test_path)
        .output()
        .expect("failed to execute process")
    };
    match output.status.code() {
        Some(x) => {
            info!("Test exited with status: {}", x.to_string());
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();

            let stdout_record_path = test_path.with_extension("k.stdout");
            let stderr_record_path = test_path.with_extension("k.stderr");
            let stdout_record: String;
            let stderr_record: String;
            if stdout_record_path.is_file() {
                info!("Record stdout for {{{}}} found", test_path.display());
                let stdout_contents = fs::read_to_string(stdout_record_path.clone());
                match stdout_contents {
                    Ok(v) => {
                        stdout_record = v;
                        trace!("Stdout record: {{\"\n{:?}\"}}", stdout_record.as_bytes());
                        trace!("Stdout found: {{\"\n{:?}\"}}", output.stdout);
                        let matching = stdout_record.as_bytes().iter().zip(output.stdout.iter()).filter(|&(a, b)| a == b).count();
                        if matching == stdout_record.len() && matching == output.stdout.len() {
                            info!("Stdout matched!");
                        } else {
                            warn!("Stdout did not match!");
                            if record {
                                info!("Recording stdout");
                                let write_res = fs::write(stdout_record_path,output.stdout);
                                match write_res {
                                    Ok(_) => {
                                        debug!("Recorded stdout");
                                    }
                                    Err(e) => {
                                        error!("Failed recording stdout. Err: {e}");
                                        return Err("Failed recording stdout".to_string());
                                    }
                                }
                            } else {
                                warn!("Expected: {{\"\n{}\"}}", stdout_record);
                                match std::str::from_utf8(&output.stdout) {
                                    Ok(v) => {
                                        info!("Found: {{\"\n{}\"}}", v);
                                        return Err("Stdout mismatch".to_string());
                                    }
                                    Err(e) => {
                                        error!("Failed parsing output.stdout. Err: {e}");
                                        return Err("Failed parsing output.stdout".to_string());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed reading stdout record for {{{}}}. Err: {e}", stdout_record_path.display());
                        return Err("Failed reading stdout record".to_string());
                    }
                }
            } else {
                warn!("Record stdout for {{{}}} not found", test_path.display());
            }

            if stderr_record_path.is_file() {
                info!("Record stderr for {{{}}} found", test_path.display());
                let stderr_contents = fs::read_to_string(stderr_record_path.clone());
                match stderr_contents {
                    Ok(v) => {
                        stderr_record = v;
                        trace!("Stderr record: {{\"\n{:?}\"}}", stderr_record.as_bytes());
                        trace!("Stderr found: {{\"\n{:?}\"}}", output.stderr);
                        let matching = stderr_record.as_bytes().iter().zip(output.stderr.iter()).filter(|&(a, b)| a == b).count();
                        if matching == stderr_record.len() && matching == output.stderr.len() {
                            info!("Stderr matched!");
                        } else if record {
                            info!("Recording stderr");
                            let write_res = fs::write(stderr_record_path,output.stderr);
                            match write_res {
                                Ok(_) => {
                                    debug!("Recorded stderr");
                                }
                                Err(e) => {
                                    error!("Failed recording stderr. Err: {e}");
                                    return Err("Failed recording stderr".to_string());
                                }
                            }
                        } else {
                            warn!("Stderr did not match!");
                            warn!("Expected: {{\"\n{}\"}}", stderr_record);
                            match std::str::from_utf8(&output.stderr) {
                                Ok(v) => {
                                    info!("Found: {{\"\n{}\"}}", v);
                                    return Err("Stderr mismatch".to_string());
                                }
                                Err(e) => {
                                    error!("Failed parsing output.stderr. Err: {e}");
                                    return Err("Failed parsing output.stderr".to_string());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed reading stderr record for {{{}}}. Err: {e}", stderr_record_path.display());
                        return Err("Failed reading stderr record".to_string());
                    }
                }
            } else {
                warn!("Record stderr for {{{}}} not found", test_path.display());
            }

            Ok("Test done".to_string())
        }
        None => {
            error!("Test command for {{{}}} failed", test_path.display());
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
            Err("Test command failed".to_string())
        }
    }
}

pub fn gen_c_header(target_path: &PathBuf, target_tag: &String, bin_name: &String) -> Result<String,String> {
    let repo = Repository::discover(target_path);
    let mut head_author_name = "".to_string();
    let id;
    let mut commit_message = "".to_string();
    let gen_time = SystemTime::now();
    let commit_time;
    let gen_timestamp = gen_time.duration_since(SystemTime::UNIX_EPOCH);
    let mut fgen_time = "".to_string();
    match gen_timestamp {
        Ok(t) => {
            fgen_time = format!("{}", t.as_secs());
        }
        Err(e) => {
            error!("Failed getting gen timestamp. Err: {e}");
        }
    }
    match repo {
        Ok(r) => {
            let lookup_name = format!("refs/tags/{}", target_tag);
            let reference = r.find_reference(&lookup_name);
            match reference {
                Ok(refr) => {
                    if !refr.is_tag() {
                        error!("{target_tag} is not a reference");
                        return Err(format!("Requested tag is not a reference: {target_tag}").to_string());
                    } else {
                        let commit = refr.peel_to_commit();
                        match commit {
                            Ok(commit) => {
                               if let Some(msg) = commit.message() {
                                   info!("Commit message: {{{}}}", msg);
                                   commit_message = msg.escape_default().to_string();
                               }
                               id = commit.id().to_string();
                               info!("Commit id: {{{}}}", id);
                               let author = commit.author();
                               let name = author.name();
                               match name {
                                  Some(name) => {
                                       head_author_name = name.to_string();
                                       info!("Commit author: {{{}}}", head_author_name);
                                   }
                                   None => {
                                       warn!("Commit author is empty: {}", head_author_name);
                                   }
                                }
                                commit_time = commit.time().seconds().to_string();
                                info!("Commit time: {{{}}}", commit_time);
                                   }
                            Err(e) => {
                                       error!("Failed peel to head commit for {{{}}}. Err: {e}", target_path.display());
                                       return Err("Failed peel to head commit for repo".to_string());
                            }
                        }
                    }
                }
                Err(_) => {
                    if target_tag.is_empty() {
                        error!("Invalid empty tag request");
                        return Err("Invalid empty tag request".to_string());
                    }
                    warn!("{}", format!("Failed getting tag {target_tag}, retrying using HEAD"));
                    let head = r.head();
                    match head {
                        Ok(head) => {
                            let commit = head.peel_to_commit();
                            match commit {
                                Ok(commit) => {
                                    if let Some(msg) = commit.message() {
                                        info!("Commit message: {{{}}}", msg);
                                        commit_message = msg.escape_default().to_string();
                                    }
                                    id = commit.id().to_string();
                                    info!("Commit id: {{{}}}", id);
                                    let author = commit.author();
                                    let name = author.name();
                                    match name {
                                        Some(name) => {
                                            head_author_name = name.to_string();
                                            info!("Commit author: {{{}}}", head_author_name);
                                        }
                                        None => {
                                            warn!("Commit author is empty: {}", head_author_name);
                                        }
                                    }
                                    commit_time = commit.time().seconds().to_string();
                                    info!("Commit time: {{{}}}", commit_time);
                                }
                                Err(e) => {
                                    error!("Failed peel to head commit for {{{}}}. Err: {e}", target_path.display());
                                    return Err("Failed peel to head commit for repo".to_string());
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed getting head for {{{}}}. Err: {e}", target_path.display());
                            return Err("Failed getting head for repo".to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed discovering repo for {{{}}}. Err: {e}", target_path.display());
            return Err("Failed discover of repo".to_string());
        }
    }
    let header_path = format!("{}/anvil__{}.h", target_path.display(), bin_name);
    trace!("Generating C header. Target path: {{{}}} Tag: {{{}}}", header_path, target_tag);
    let output = File::create(header_path);
    let header_string = format!("//Generated by invil v{INVIL_VERSION}\n
//Repo at https://github.com/jgabaut/invil\n
#ifndef ANVIL__{bin_name}__\n
#define ANVIL__{bin_name}__\n
static const char ANVIL__API_LEVEL__STRING[] = \"{EXPECTED_AMBOSO_API_LEVEL}\"; /**< Represents amboso version used for [anvil__{bin_name}.h] generated header.*/\n
static const char ANVIL__{bin_name}__VERSION_STRING[] = \"{target_tag}\"; /**< Represents current version for [anvil__{bin_name}.h] generated header.*/\n
static const char ANVIL__{bin_name}__VERSION_DESC[] = \"{id}\"; /**< Represents current version info for [anvil__{bin_name}.h] generated header.*/\n
static const char ANVIL__{bin_name}__VERSION_DATE[] = \"{commit_time}\"; /**< Represents date for current version for [anvil__{bin_name}.h] generated header.*/\n
static const char ANVIL__{bin_name}__VERSION_AUTHOR[] = \"{head_author_name}\"; /**< Represents author for current version for [anvil__{bin_name}.h] generated header.*/\n
static const char ANVIL__{bin_name}__HEADER_GENTIME[] = \"{fgen_time}\"; /**< Represents gen time for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__API__LEVEL__(void); /**< Returns a version string for amboso API of [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__(void); /**< Returns a version string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__DESC__(void); /**< Returns a version info string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__DATE__(void); /**< Returns a version date string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__AUTHOR__(void); /**< Returns a version author string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__HEADER__GENTIME__(void); /**< Returns a string for time of gen for [anvil__{bin_name}.h] generated header.*/\n
#ifndef INVIL__{bin_name}__HEADER__
#define INVIL__{bin_name}__HEADER__
static const char INVIL__VERSION__STRING[] = \"{INVIL_VERSION}\"; /**< Represents invil version used for [anvil__{bin_name}.h] generated header.*/\n
static const char INVIL__OS__STRING[] = \"{INVIL_OS}\"; /**< Represents build os used for [anvil__{bin_name}.h] generated header.*/\n
static const char INVIL__COMMIT__DESC__STRING[] = \"{commit_message}\"; /**< Represents message for HEAD commit used for [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__API__LEVEL__(void); /**< Returns a version string for invil version of [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__OS__(void); /**< Returns a version string for os used for [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__COMMIT__DESC__(void); /**< Returns a string for HEAD commit message used for [anvil__{bin_name}.h] generated header.*/\n
#endif // INVIL__{bin_name}__HEADER__
#endif\n");
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
    let c_impl_string = format!("//Generated by invil v{INVIL_VERSION}\n
#include \"anvil__{bin_name}.h\"\n
const char *get_ANVIL__VERSION__(void)
{{
    return ANVIL__{bin_name}__VERSION_STRING;
}}\n
const char *get_ANVIL__API__LEVEL__(void)
{{
    return ANVIL__API_LEVEL__STRING;
}}\n
const char *get_ANVIL__VERSION__DESC__(void)
{{
    return ANVIL__{bin_name}__VERSION_DESC;
}}\n
const char *get_ANVIL__VERSION__DATE__(void)
{{
    return ANVIL__{bin_name}__VERSION_DATE;
}}\n
const char *get_ANVIL__VERSION__AUTHOR__(void)
{{
    return ANVIL__{bin_name}__VERSION_AUTHOR;
}}\n
const char *get_ANVIL__HEADER__GENTIME__(void)
{{
    return ANVIL__{bin_name}__HEADER_GENTIME;
}}\n
#ifdef INVIL__{bin_name}__HEADER__
const char *get_INVIL__API__LEVEL__(void)
{{
    return INVIL__VERSION__STRING;
}}\n
const char *get_INVIL__COMMIT__DESC__(void)
{{
    return INVIL__COMMIT__DESC__STRING;
}}\n
const char *get_INVIL__OS__(void)
{{
    return INVIL__OS__STRING;
}}
#endif\n");
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

fn try_lex_makefile(file_path: impl AsRef<Path>, dbg_print: bool, skip_recap: bool, report_warns: bool) -> Result<String,String> {
    warn!("Makefile parsing is experimental.");
    let path = file_path.as_ref();
    let res = lex_makefile(path, dbg_print, skip_recap, report_warns);
    match res {
        Ok(warns) => {
            if warns != 0 {
                trace!("Failed lex for {{{}}}.\nTot warns: {warns}.", path.display());
                return Err(format!("Lex failure, {warns} warnings."));
            }
            debug!("Lex successful for {{{}}}.", path.display());
            Ok(format!("Lex success with {warns} warns."))
        }
        Err(e) => {
            trace!("Failed lex for {{{}}}.\nError was:    {e}", path.display());
            Err("Lex failure".to_string())
        }
    }
}

pub fn handle_linter_flag(stego_path: &PathBuf, lint_mode: &AmbosoLintMode) -> Result<String,String> {
    debug!("Linter for file: {{{}}}", stego_path.display());
    if stego_path.exists() {
        trace!("Found {}", stego_path.display());
        match lint_mode {
            AmbosoLintMode::NajloFull => {
                try_lex_makefile(stego_path, false, false, true)
            }
            AmbosoLintMode::NajloDebug => {
                try_lex_makefile(stego_path, true, false, true)
            }
            AmbosoLintMode::NajloQuiet => {
                try_lex_makefile(stego_path, false, true, true)
            }
            AmbosoLintMode::LintOnly => {
                let res = try_parse_stego(stego_path);
                match res {
                    Ok(_) => {
                        info!("Lint successful for {{{}}}.", stego_path.display());
                        Ok("Lint success".to_string())
                }
                    Err(e) => {
                        trace!("Failed lint for {{{}}}.\nError was:    {e}",stego_path.display());
                        Err("Lint failure".to_string())
                    }
                }
            }
            AmbosoLintMode::FullCheck => {
                let res = parse_stego_toml(stego_path, &PathBuf::from(""));
                match res {
                    Ok(r) => {
                        info!("Lint successful for {{{}}}.", stego_path.display());
                        println!("ANVIL_AUTOMAKE_VERS: {{{}}}", r.mintag_automake.unwrap_or("".to_string()));
                        println!("ANVIL_MAKE_VERS: {{{}}}", r.mintag_make.unwrap_or("".to_string()));
                        println!("ANVIL_BIN: {{{}}}", r.bin.unwrap_or("".to_string()));
                        println!("ANVIL_SOURCE: {{{}}}", r.source.unwrap_or("".to_string()));
                        println!("ANVIL_TESTDIR: {{{}}}", r.tests_dir.unwrap_or(PathBuf::from("")).display());
                        println!("ANVIL_BONE_DIR: {{{}}}", r.bonetests_dir.unwrap_or(PathBuf::from("")).display());
                        println!("ANVIL_KULPO_DIR: {{{}}}", r.kulpotests_dir.unwrap_or(PathBuf::from("")).display());
                        for bv in r.basemode_versions_table {
                            println!("ANVIL_BASE_VERSION: {{{}}}", bv.0);
                        }
                        for v in r.gitmode_versions_table {
                            println!("ANVIL_GIT_VERSION: {{{}}}", v.0);
                        }
                        Ok("Full linter check success".to_string())
                    }
                    Err(e) => {
                        error!("Failed lint for {{{}}}.\nError was:    {e}", stego_path.display());
                        Err(e)
                    }
                }
            }
            AmbosoLintMode::Lex => {
                let res = lex_stego_toml(stego_path);
                match res {
                    Ok(_) => {
                        debug!("Lex successful for {{{}}}.", stego_path.display());
                        Ok("Linter lex check success".to_string())
                    }
                    Err(e) => {
                        error!("Failed lex for {{{}}}.\nError was:    {e}",stego_path.display());
                        Err(e)
                    }
                }
            }
        }
    } else {
        error!("Could not find file: {{{}}}", stego_path.display());
        Err("Failed linter call".to_string())
    }
}

pub fn handle_running_make() {
    if cfg!(target_os = "windows") {
        todo!("Support windows make run?");
        /*
         * let output = Command::new("cmd")
         *   .args(["/C", "echo hello"])
         *   .output()
         *   .expect("failed to execute process")
         */
    } else if Path::new("./Makefile").exists() {
        info!("Found Makefile");
        debug!("Running \'make\'");
        let output = Command::new("make")
            .output()
            .expect("failed to execute process");

        match output.status.code() {
            Some(make_ec) => {
                if make_ec == 0 {
                    debug!("make succeded with status: {}", make_ec.to_string());
                    exit(make_ec);
                } else {
                    error!("make failed with status: {}", make_ec.to_string());
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();
                    exit(make_ec);
                }
            }
            None => {
                error!("make command failed");
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                exit(1);
            }
        }
    } else if Path::new("./configure.ac").exists() && Path::new("./Makefile.am").exists() {
        debug!("Running \'aclocal ; autoconf; automake --add-missing ; ./configure; make\'");
        let output = Command::new("sh")
            .arg("-c")
            .arg("aclocal ; autoconf ; automake --add-missing ; ./configure; make")
            .output()
            .expect("failed to execute process");

        match output.status.code() {
            Some(autotools_prep_ec) => {
                if autotools_prep_ec == 0 {
                    debug!("Automake prep succeded with status: {}", autotools_prep_ec.to_string());
                    exit(autotools_prep_ec);
                } else {
                    error!("Automake failed with status: {}", autotools_prep_ec.to_string());
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();
                    exit(autotools_prep_ec);
                }
            }
            None => {
                error!("Automake prep command failed");
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                exit(1);
            }
        }
    } else {
        error!("Can't find Makefile or configure.ac and Makefile.am. Quitting.");
        exit(1);
    }
}

pub fn lex_makefile(file_path: impl AsRef<Path>, dbg_print: bool, skip_recap: bool, report_warns: bool) -> io::Result<u64> {
    let path = file_path.as_ref();

    // Check if the file exists
    if !path.exists() {
        error!("File not found: {}", path.display());
        std::process::exit(1);
    }

    let mut last_rulename: String = "".to_string();
    let mut _ingr_i: u64 = 0;
    let mut mainexpr_arr: Vec<String> = Vec::new();
    let mut rules_arr: Vec<String> = Vec::new();
    let mut ruleingrs_arr: Vec<String> = Vec::new();
    let mut rulexpr_arr: Vec<String> = Vec::new();
    let mut tot_warns: u64 = 0;
    let mut cur_line: u64 = 0;
    let mut rule_i: usize = 0;
    let mut rulexpr_i: u64 = 0;
    let mut mainexpr_i: usize = 0;
    // Read the file line by line
    if let Ok(file) = File::open(path) {
        let tab_regex = Regex::new(&format!("^{}", RULELINE_MARK_CHAR)).expect("Failed to create ruleline regex");
        let rule_regex = Regex::new(RULE_REGEX).expect("Failed to create rule regex");

        let mut continuation = false;
        let mut current_line = String::new();

        let rulewarn_regex = Regex::new(RULEWARN_REGEX).expect("Failed to create rulewarn regex");
        for line in io::BufReader::new(file).lines().map_while(Option::Some).flatten() {
            if continuation {
                current_line.pop();
                current_line.push_str(&line);
            } else {
                current_line = line;
            }
            continuation = current_line.ends_with("\\");

            if continuation {
                continue
            }
            cur_line += 1;
            if current_line.is_empty() {
                trace!("Ignoring empty line.");
                continue;
            }
            let _comment = cut_line_at_char(&current_line, '#', CutDirection::After);
            let stripped_line = cut_line_at_char(&current_line, '#', CutDirection::Before);
            if rule_regex.is_match(stripped_line) {
                let rulename = cut_line_at_char(stripped_line, ':', CutDirection::Before);
                let mut rule_ingredients = cut_line_at_char(cut_line_at_char(stripped_line, ':', CutDirection::After), ' ', CutDirection::After); // Cut ': '
                let mut ingrs_len = 0;
                rulexpr_i = 0;
                ruleingrs_arr.push("".to_string());
                rulexpr_arr.push("".to_string());
                let set_len: bool = rule_ingredients.is_empty();
                if set_len {
                    rule_ingredients = "NO_DEPS";
                    ingrs_len = 0;
                }
                let ingrs_arr: Vec<_> = rule_ingredients.split_whitespace().collect();
                if !set_len {
                    ingrs_len = ingrs_arr.len();
                }
                //println!("Line matches rule regex: {}", stripped_line);
                last_rulename = rulename.to_string();
                let mod_time: String;
                let rule_path = Path::new(rulename);
                if rule_path.exists() {
                    let metadata = fs::metadata(rulename)?;

                    if let Ok(time) = metadata.modified() {
                        let mod_timestamp = time.duration_since(SystemTime::UNIX_EPOCH);
                        match mod_timestamp {
                            Ok(t) => {
                                mod_time=format!("{}", t.as_secs());
                            }
                            Err(e) => {
                                panic!("Failed getting modification time for {}. Err: {e}", rule_path.display());
                            }
                        }
                    } else {
                        // This branch is meant for unsupported platforms.
                        mod_time="NO_TIME".to_string();
                    }
                } else {
                    mod_time="NO_TIME".to_string();
                }
                let rulepart_decl = format!("{{RULE}} [#{rule_i}] -> {{{rulename}}} <- {{{mod_time}}}");
                let rulepart_deps = if !set_len {
                    format!("<- {{DEPS}} -> {{{rule_ingredients}}} -> [#{ingrs_len}]")
                } else {
                    format!("<- {{DEPS}} -> {{}} -> [#{ingrs_len}]")
                };
                //let rule_str = format!("{{RULE}} [#{rule_i}] -> {{{rulename}}} <- {{{mod_time}}} <- {{DEPS}} -> {{{rule_ingredients}}} -> [#{ingrs_len}]");
                let rule_str = format!("{rulepart_decl} {rulepart_deps}");
                rules_arr.push(rule_str.clone());
                if dbg_print {
                    println!("{rulepart_decl}\n\t{rulepart_deps} ->");
                }
                let mut ingr_mod_time: String;
                if !set_len {
                    for (ingr_i, ingr) in ingrs_arr.iter().enumerate() {
                        let ingr_path = Path::new(ingr);
                        if ingr_path.exists() {
                            //TODO
                            //In amboso 2.0.3, the embedded najlo version (0.0.3) wrongly
                            //passes the rule name to the modification time call. A strict port
                            //would do the same?
                            let ingr_metadata = fs::metadata(ingr_path)?;

                            if let Ok(time) = ingr_metadata.modified() {
                                let mod_timestamp = time.duration_since(SystemTime::UNIX_EPOCH);
                                match mod_timestamp {
                                    Ok(t) => {
                                        ingr_mod_time=format!("{}", t.as_secs());
                                    }
                                    Err(e) => {
                                        panic!("Failed getting modification time for {}. Err: {e}", ingr_path.display());
                                    }
                                }
                            } else {
                                // This branch is meant for unsupported platforms.
                                ingr_mod_time="NO_TIME".to_string();
                            }
                        } else {
                            ingr_mod_time="NO_TIME".to_string();
                        }

                        let ingr_str = format!("{{{ingr}}} {{[{ingr_i}], [{ingr_mod_time}]}}, ");
                        if dbg_print {
                            println!("\t\t{{INGR}} - {{{ingr}}} [{ingr_i}], [{ingr_mod_time}]");
                        }
                        ruleingrs_arr[rule_i] = format!("{}{ingr_str}", ruleingrs_arr[rule_i]);
                    }
                    ruleingrs_arr[rule_i] = format!("{{RULE: {rulename} #{rule_i}}} <-- [{}]", ruleingrs_arr[rule_i]);
                } else {
                    if dbg_print {
                        println!("\t\t{{{rule_ingredients}}}");
                    }
                    ruleingrs_arr[rule_i] = format!("{{RULE: {rulename} #{rule_i}}} <-- [{{NO_DEPS}}]");
                }
                if dbg_print {
                    println!("\t}};");
                }
                rule_i += 1;
            } else if !last_rulename.is_empty() && tab_regex.is_match(stripped_line) {
                //println!("Line starts with a tab: {}", stripped_line);
                let stripped_rulexpr_line = cut_line_at_char(stripped_line, '\t', CutDirection::After);
                if stripped_rulexpr_line.is_empty() {
                    trace!("Ignoring empty stripped rulexpr line.");
                    continue;
                }
                if dbg_print {
                    println!("\t{{RULE_EXPR}} -> {{{stripped_rulexpr_line}}}, [#{rulexpr_i}],");
                }
                if rulexpr_arr.is_empty() {
                    error!("Can't have this. Line: [#{cur_line}], Stripped rulexpr line: {{{stripped_rulexpr_line}}}");
                    panic!("OUCH");
                }
                let rulexpr_str = format!("{{RULE_EXPR #{rulexpr_i}}} {{{stripped_rulexpr_line}}}, ");
                rulexpr_arr[rule_i-1] = format!("{}{}", rulexpr_arr[rule_i-1], rulexpr_str);
                rulexpr_i += 1;
            } else {
                //println!("Line does not start with a tab: {}", stripped_line);
                if stripped_line.is_empty() {
                    trace!("Ignoring empty stripped line.");
                    continue;
                } else {
                    rulexpr_i = 0;
                }
                if last_rulename.is_empty() {
                    if tab_regex.is_match(stripped_line) {
                        // This branch is not 1-1 in najlo, but it's needed
                        //
                        // TODO
                        // Correctly concatenate the expressions, in some way
                        // Current implementation may be a bit clunky but is close
                        //
                        debug!("Found mainexpr starting with a tab. {{{stripped_line}}}");
                        todo!("Implement handling tabbed main_exprs");
                        /*
                        let mainexpr_str = format!("{{EXPR_MAIN}} -> {{{stripped_line}}}, [#{mainexpr_i}]");
                        if dbg_print {
                            println!("{},", mainexpr_str);
                        }
                        mainexpr_arr[mainexpr_i-1] = format!("{}{}", mainexpr_arr[mainexpr_i-1], mainexpr_str);
                        continue;
                        */
                    }
                    //println!("Line is an expression before any rule was found");
                    let mainexpr_str = format!("{{EXPR_MAIN}} -> {{{stripped_line}}}, [#{mainexpr_i}]");
                    if dbg_print {
                        println!("{},", mainexpr_str);
                    }
                    mainexpr_arr.push(mainexpr_str);
                    mainexpr_i += 1;
                } else {
                    //println!("Line is an expression after at least one rule was found");
                    let mainexpr_str = format!("{{EXPR_MAIN}} -> {{{stripped_line}}}, [#{mainexpr_i}]");
                    if dbg_print {
                        println!("{},", mainexpr_str);
                    }
                    if report_warns && rulewarn_regex.is_match(stripped_line) {
                        warn!("A recipe line must start with a tab.");
                        warn!("{stripped_line}");
                        warn!("^^^ Any recipe line starting with a space will be interpreted as a main expression.");
                        tot_warns += 1;
                    }
                    mainexpr_arr.push(mainexpr_str);
                    mainexpr_i += 1;
                }
            }
        }

        if skip_recap { return Ok(tot_warns); }

        println!("{{MAIN}} -> {{");
        for mexpr in mainexpr_arr {
            println!("\t[{}],", mexpr);
        }
        println!("}}");
        println!("{{RULES}} -> {{");
        for rule in &rules_arr {
            println!("\t[{}],", rule);
        }
        println!("}}");
        println!("{{DEPS}} -> {{");
        for ruleingr in ruleingrs_arr {
            println!("\t[{}],", ruleingr);
        }
        println!("}}");
        println!("{{RULE_EXPRS}} -> {{");
        for (i, rulexpr) in rulexpr_arr.iter().enumerate() {
            println!("\t[[{}] --> [{rulexpr}]],", rules_arr[i]);
        }
        println!("}}");
    } else {
        eprintln!("Failed to open file: {}", path.display());
        std::process::exit(1);
    }

    Ok(tot_warns)
}

fn build_step(args: &Args, env: &AmbosoEnv, cflg_str: String, query: &str, bin_path: PathBuf, build_path: PathBuf, bin: String, head_was_detached: bool) -> Result<String,String> {
    let output;
    let build_step_command;
    match env.anvil_kern {
        AnvilKern::AmbosoC => {
            build_step_command = "make";
        }
        AnvilKern::AnvilPy => {
            let mut use_python_build = true;
            if args.strict {
                match semver_compare(&env.anvil_version, MIN_AMBOSO_V_PYKERN) {
                    Ordering::Less => {
                        warn!("Strict behaviour for v{}, still using make", env.anvil_version);
                        use_python_build = false;
                    }
                    Ordering::Equal | Ordering::Greater => {}
                }
            }
            if use_python_build {
                build_step_command = "python -m build"; // Using -o bin_path would allow skipping the
                                                        // mv command
            } else {
                build_step_command = "make";
            }
        }
        AnvilKern::Custom => {
            #[cfg(feature = "anvilCustom")] {
                match &env.anvilcustom_env {
                    Some(cust_env) => {
                        build_step_command = &cust_env.custom_builder;
                    }
                    None => {
                        error!("Missing anvilcustom_env");
                        return Err("Missing anvilcustom_env".to_string());
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

    match env.anvil_kern {
        AnvilKern::AmbosoC => {
            if args.no_rebuild {
                debug!("Running \'{build_step_command}\'");
                output = Command::new(build_step_command)
                    .arg(cflg_str)
                    .output()
                    .expect("failed to execute process");
            } else {
                debug!("Running \'make rebuild\'");
                output = Command::new("make")
                    .arg("rebuild")
                    .arg(cflg_str)
                    .output()
                    .expect("failed to execute process");
            }
        }
        AnvilKern::AnvilPy => {
            debug!("Running \'{build_step_command}\'");
            output = Command::new(build_step_command)
                .arg(cflg_str)
                .output()
                .expect("failed to execute process");
        }
        AnvilKern::Custom => {
            // "custom_builder" "target_d" "bin_name" "q_tag" "stego_dir"
            let custom_call = format!("{} {} {} {} {}", build_step_command,
                build_path.display(),
                bin,
                query,
                env.stego_dir.clone().expect("failed initialising stego_dir").display()
            );
            debug!("Running \'{custom_call}\'");
            output = Command::new(build_step_command)
                .arg(build_path.clone())
                .arg(bin.clone())
                .arg(query)
                .arg(env.stego_dir.clone().expect("failed initialising stego_dir"))
                .output()
                .expect("failed to execute process");
        }
    }
    match output.status.code() {
        Some(make_ec) => {
            if make_ec == 0 {
               debug!("{{{}}} succeded with status: {}", build_step_command, make_ec.to_string());
               match env.run_mode.as_ref().unwrap() {
                   AmbosoMode::GitMode => {
                       postbuild_step(env, query, bin_path, build_path, bin, head_was_detached)
                   }
                   _ => {
                       trace!("Avoiding postbuild_step outside of GitMode");
                       Ok(format!("{{{build_step_command}}} succeded"))
                   }
               }
            } else {
                warn!("{{{}}} failed with status: {}", build_step_command, make_ec.to_string());
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                Err(format!("{{{build_step_command}}} failed"))
            }
        }
        None => {
            error!("{{{}}} command failed", build_step_command);
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
            Err(format!("{{{build_step_command}}} command failed"))
        }
    }
}

fn git_switch_and_submodule_init_re(query: &str, head_was_detached: bool) -> Result<String,String> {
    trace!("Running \'git switch -\'");
    if head_was_detached {
        debug!("Checkout started from a detached HEAD, will add --detach to the switchback");
    }
    let output = match head_was_detached {
        false => {
            Command::new("git")
            .arg("switch")
            .arg("-")
            .output()
            .expect("failed to execute process")
        }
        true => {
            Command::new("git")
            .arg("switch")
            .arg("-")
            .arg("--detach")
            .output()
            .expect("failed to execute process")
        }
    };
    match output.status.code() {
        Some(gswitch_ec) => {
            if gswitch_ec == 0 {
               debug!("git switch succeded with status: {}", gswitch_ec.to_string());
                trace!("Running \'git submodule update --init --recursive\'");
                let output = Command::new("git")
                    .arg("submodule")
                    .arg("update")
                    .arg("--init")
                    .arg("--recursive")
                    .output()
                    .expect("failed to execute process");
                match output.status.code() {
                    Some(gsinit_end_ec) => {
                        if gsinit_end_ec == 0 {
                            debug!("git submodule init succeded with status: {}", gsinit_end_ec.to_string());
                            debug!("Done build for {}", query);
                            Ok(format!("Done build step for {{{query}}}"))
                        } else {
                            warn!("git submodule init failed with status: {}", gsinit_end_ec.to_string());
                            io::stdout().write_all(&output.stdout).unwrap();
                            io::stderr().write_all(&output.stderr).unwrap();
                            Err("git submodule init failed".to_string())
                        }
                    }
                    None => {
                        error!("git submodule init command failed");
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        Err("git submodule init command failed".to_string())
                    }
                }
            } else {
                warn!("git switch failed with status: {}", gswitch_ec.to_string());
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                Err("git switch failed".to_string())
            }
        }
        None => {
            error!("git switch command failed");
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
            Err("git switch command failed".to_string())
        }
    }
}

fn postbuild_step(env: &AmbosoEnv, query: &str, bin_path: PathBuf, build_path: PathBuf, bin: String, head_was_detached: bool) -> Result<String,String> {

    let output;
    match env.anvil_kern {
        AnvilKern::AmbosoC => {
            trace!("Running \'mv {} {}\'", bin, bin_path.display());
            output = Command::new("mv")
                .arg(bin)
                .arg(bin_path)
                .output()
                .expect("failed to execute process");
        }
        AnvilKern::AnvilPy => {
            #[cfg(feature = "anvilPy")] {
                let curr_proj_name = env.anvilpy_env.as_ref().expect("Failed initialising anvilpy_env").proj_name.clone().replace("-","_");
                let srcdist_name = format!("{}-{}.tar.gz", curr_proj_name, query);
                let mut srcdist_path = PathBuf::from("./dist/");
                srcdist_path.push(srcdist_name.clone());
                info!("curr_proj_name {} srcdist_name {}", curr_proj_name, srcdist_name);
                trace!("Running \'mv {} {}\'", srcdist_path.display(), build_path.display());
                let output_srcdist = Command::new("mv")
                    .arg(srcdist_path)
                    .arg(build_path.clone())
                    .output()
                    .expect("failed to execute process");
                match output_srcdist.status.code() {
                    Some(mv_ec) => {
                        if mv_ec == 0 {
                            debug!("mv srcdist succeded with status: {}", mv_ec.to_string());
                            let mut srcdist_pack_path = PathBuf::from(build_path.clone());
                            srcdist_pack_path.push(srcdist_name);
                            let unpack_res = unpack_srcdist(&srcdist_pack_path);
                            match unpack_res {
                                Ok(unpack_path) => {
                                    let proj_dirname = format!("{curr_proj_name}-{query}");
                                    let mut target_unpack_path = unpack_path.clone();
                                    target_unpack_path.push(ANVILPY_UNPACKDIR_NAME);
                                    let mut curr_unpack_path = unpack_path.clone();
                                    curr_unpack_path.push(proj_dirname.clone());

                                    trace!("Running \'mv {} {}\'", curr_unpack_path.display(), target_unpack_path.display());
                                    let output_unpackmv = Command::new("mv")
                                        .arg(curr_unpack_path.clone())
                                        .arg(target_unpack_path.clone())
                                        .output()
                                        .expect("failed to execute process");
                                    match output_unpackmv.status.code() {
                                        Some(mv_ec) => {
                                            if mv_ec == 0 {
                                                trace!("Moved {{{}}} to {{{}}}", curr_unpack_path.display(), target_unpack_path.display());
                                            } else {
                                                warn!("mv unpack failed with status: {}", mv_ec.to_string());
                                                io::stdout().write_all(&output_unpackmv.stdout).unwrap();
                                                io::stderr().write_all(&output_unpackmv.stderr).unwrap();
                                                return Err("mv unpack failed".to_string());
                                            }
                                        }
                                        None => {
                                            error!("mv unpack command failed");
                                            io::stdout().write_all(&output_srcdist.stdout).unwrap();
                                            io::stderr().write_all(&output_srcdist.stderr).unwrap();
                                            return Err("mv command failed".to_string());
                                        }
                                    }
                                    let mut unpack_initpy_path = target_unpack_path.clone();
                                    unpack_initpy_path.push("__init__.py");
                                    match post_unpack(&unpack_initpy_path, &build_path, &env) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            error!("Failed post_unpack() for {{{}}}. Err: {e}", target_unpack_path.display());
                                            return Err("Failed post_unpack()".to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        } else {
                            warn!("mv srcdist failed with status: {}", mv_ec.to_string());
                            io::stdout().write_all(&output_srcdist.stdout).unwrap();
                            io::stderr().write_all(&output_srcdist.stderr).unwrap();
                            return Err("mv failed".to_string());
                        }
                    }
                    None => {
                        error!("mv srcdist command failed");
                        io::stdout().write_all(&output_srcdist.stdout).unwrap();
                        io::stderr().write_all(&output_srcdist.stderr).unwrap();
                        return Err("mv command failed".to_string());
                    }
                }
                let curr_whldist_name = format!("./dist/{}-{}-py3-none-any.whl", curr_proj_name, query);
                let curr_whldist_path = PathBuf::from(curr_whldist_name);
                trace!("Running \'mv {} {}\'", curr_whldist_path.display(), build_path.display());
                output = Command::new("mv")
                    .arg(curr_whldist_path)
                    .arg(build_path)
                    .output()
                    .expect("failed to execute process");
            }
            #[cfg(not(feature = "anvilPy"))] {
                // Handle AnvilPy case when the feature is not enabled
                error!("AnvilPy kern feature is not enabled. Can't build {} to {}", bin, build_path.display());
                return Err("AnvilPy kern feauture is not enabled".to_string());
            }
        }
        AnvilKern::Custom => {
            #[cfg(feature = "anvilCustom")] {
                if !bin_path.exists() {
                } else {
                    debug!("It seems that custom command created {{{}}}.", bin_path.display());
                    debug!("Ignoring the move step.");
                    match env.run_mode.as_ref().unwrap() {
                        AmbosoMode::GitMode => {
                            let gswinit_res = git_switch_and_submodule_init_re(query);
                            match gswinit_res {
                                Ok(_) => {
                                    trace!("Done git cleaning");
                                    return Ok("Custom command moved the build by itself".to_string());
                                }
                                Err(e) => {
                                    error!("git cleaning failed");
                                    return Err(e);
                                }
                            }
                        }
                        _ => {
                            error!("Unexpected mode in postbuild_step(): {:?}", env.run_mode.as_ref());
                            return Err("Unexpected mode in postbuild step".to_string());
                        }
                    }
                }
                trace!("TODO: postbuild checks for custom kern");
                trace!("Running \'mv {} {}\'", bin, bin_path.display());
                output = Command::new("mv")
                    .arg(bin)
                    .arg(bin_path)
                    .output()
                    .expect("failed to execute process");
            }
            #[cfg(not(feature = "anvilCustom"))] {
                // Handle AnvilCustom case when the feature is not enabled
                error!("AnvilCustom kern feature is not enabled. Can't build {} to {}", bin, build_path.display());
                return Err("AnvilCustom kern feauture is not enabled".to_string());
            }
        }
    }

    match env.anvil_kern {
        AnvilKern::AmbosoC | AnvilKern::AnvilPy | AnvilKern::Custom => {
            match output.status.code() {
                Some(mv_ec) => {
                    if mv_ec == 0 {
                        debug!("mv succeded with status: {}", mv_ec.to_string());
                        match env.run_mode.as_ref().unwrap() {
                            AmbosoMode::GitMode => {
                                let gswinit_res = git_switch_and_submodule_init_re(query, head_was_detached);
                                match gswinit_res {
                                    Ok(m) => {
                                        trace!("Done git cleaning");
                                        Ok(m)
                                    }
                                    Err(e) => {
                                        error!("git cleaning failed");
                                        Err(e)
                                    }
                                }
                            }
                            _ => {
                                error!("Unexpected mode in postbuild_step(): {:?}", env.run_mode.as_ref());
                                Err("Unexpected mode in postbuild step".to_string())
                            }
                        }
                    } else {
                        warn!("mv failed with status: {}", mv_ec.to_string());
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        Err("mv failed".to_string())
                    }
                }
                None => {
                    error!("mv command failed");
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();
                    Err("mv command failed".to_string())
                }
            }
        }
    }
}
