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
use crate::core::{Args, AmbosoEnv, AmbosoMode, INVIL_VERSION, INVIL_OS, EXPECTED_AMBOSO_API_LEVEL};
use std::process::Command;
use std::io::{self, Write};
use std::path::PathBuf;
use is_executable::is_executable;
use std::collections::BTreeMap;
use std::fs::{self, File};
use git2::Repository;

pub fn do_build(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
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
                                    .arg(format!("aclocal ; autoconf ; automake --add-missing ; ./configure \"{}\"", env.configure_arg))
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

pub fn do_run(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
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

pub fn do_delete(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
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

pub fn do_query(env: &AmbosoEnv, args: &Args) -> Result<String,String> {
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
                    if ! env.support_testmode {
                        return Err("Missing testmode support".to_string());
                    } else {
                        let do_record : bool;
                        if env.do_build {
                            info!("Recording test {{{:?}}}", q);
                            do_record = true;
                        } else {
                            info!("Testing {{{:?}}}", q);
                            do_record = false;
                        }

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
                                    if is_executable(&qp) {
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
                            return Ok("Is executable".to_string());
                        } else {
                            debug!("{} is not executable", queried_path.display());
                            return Ok("Is not executable".to_string());
                        }

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
            match env.run_mode.as_ref().unwrap() {
                AmbosoMode::TestMacro => {
                    if ! env.support_testmode {
                        return Err("Missing testmode support".to_string());
                    } else {
                        let do_record : bool;
                        if env.do_build {
                            info!("Recording all tests");
                            do_record = true;
                        } else {
                            info!("Running all tests");
                            do_record = false;
                        }
                        let mut alltests_map: BTreeMap<String, PathBuf> = BTreeMap::new();
                        let mut bonetests_map = env.bonetests_table.clone();
                        let mut kulpotests_map = env.kulpotests_table.clone();
                        alltests_map.append(&mut bonetests_map);
                        alltests_map.append(&mut kulpotests_map);
                        for test in alltests_map.values() {
                            if test.exists() {
                                trace!("Found {{{}}}", test.display());
                                if test.is_file() {
                                    info!("{} is a file", test.display());
                                    if is_executable(&test) {
                                        debug!("{} is executable", test.display());
                                        let test_res = run_test(test, do_record);

                                        if args.watch {
                                            let test_elapsed = env.start_time.elapsed();
                                            info!("Done test {{{}}}, Elapsed: {:.2?}", test.display(), test_elapsed);
                                        }
                                        info!("Test cmd: {{{:?}}}", test_res);
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
                        info!("Done test macro");
                        return Ok("Done test macro run".to_string());
                    }
                }
                _ => {
                }
            }
            warn!("No tag provided for query op.");
            return Err("No tag provided.".to_string())
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
        Command::new("sh")
        .arg("-c")
        .arg(format!("{}", test_path.display()))
        .output()
        .expect("failed to execute process")
    };
    match output.status.code() {
        Some(x) => {
            if x == 0 {
                info!("Test exited with status: {}", x.to_string());
            } else {
                info!("Test exited with status: {}", x.to_string());
            }
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
                        if matching == stdout_record.as_bytes().len() && matching == output.stdout.len() {
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
                                info!("Expected: {{\"\n{}\"}}", stdout_record);
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
                        if matching == stderr_record.as_bytes().len() && matching == output.stderr.len() {
                            info!("Stderr matched!");
                        } else {
                            warn!("Stderr did not match!");
                            info!("Expected: {{\"\n{}\"}}", stderr_record);
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

            return Ok("Test done".to_string());
        }
        None => {
            error!("Test command for {{{}}} failed", test_path.display());
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
            return Err("Test command failed".to_string());
        }
    }
}

pub fn gen_c_header(target_path: &PathBuf, target_tag: &String, bin_name: &String) -> Result<String,String> {
    let repo = Repository::discover(target_path);
    let mut head_author_name = "".to_string();
    let id;
    let commit_time;
    let mut commit_message = "".to_string();
    match repo {
        Ok(r) => {
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
                            commit_time = commit.time().seconds();
                            info!("Commit time: {{{}}}", commit_time);
                               }
                               Err(e) => {
                                   error!("Failed peel to head commit for {{{}}}. Err: {e}", target_path.display());
                                   return Err("Failed peel to head commit for repo".to_string());
                               }
                            }
                }
                Err(e) => {
                    error!("Failed getting head for {{{}}}. Err: {e}", target_path.display());
                    return Err("Failed getting head for repo".to_string());
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
const char *get_ANVIL__API__LEVEL__(void); /**< Returns a version string for amboso API of [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__(void); /**< Returns a version string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__DESC__(void); /**< Returns a version info string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__DATE(void); /**< Returns a version date string for [anvil__{bin_name}.h] generated header.*/\n
const char *get_ANVIL__VERSION__AUTHOR(void); /**< Returns a version author string for [anvil__{bin_name}.h] generated header.*/\n
#ifndef INVIL__{bin_name}__HEADER__
#define INVIL__{bin_name}__HEADER__
static const char INVIL__VERSION__STRING[] = \"{INVIL_VERSION}\"; /**< Represents invil version used for [anvil__{bin_name}.h] generated header.*/\n
static const char INVIL__OS__STRING[] = \"{INVIL_OS}\"; /**< Represents build os used for [anvil__{bin_name}.h] generated header.*/\n
static const char INVIL__COMMIT__DESC__STRING[] = \"{commit_message}\"; /**< Represents message for HEAD commit used for [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__API__LEVEL__(void); /**< Returns a version string for invil version of [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__OS__(void); /**< Returns a version string for os used for [anvil__{bin_name}.h] generated header.*/\n
const char *get_INVIL__COMMIT__DESC__(void); /**< Returns a string for HEAD commit message used for [anvil__{bin_name}.h] generated header.*/\n
#endif // INVIL__{bin_name}__HEADER__
#endif");
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
#endif");
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
