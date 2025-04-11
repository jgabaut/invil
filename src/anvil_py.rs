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
use url::{Url};

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path,PathBuf};
use std::time::Instant;
use toml::Table;
use tar::Archive;
use flate2::read::GzDecoder;
use std::os::unix::fs::PermissionsExt;
use std::fs::OpenOptions;

use crate::core::{INVIL_VERSION, AmbosoEnv};

pub const ANVILPY_PROJECT_VERSION_KEYNAME: &str = "version";
pub const ANVILPY_PROJECT_NAME_KEYNAME: &str = "name";
pub const ANVILPY_PROJECT_DESC_KEYNAME: &str = "description";
pub const ANVILPY_PROJECT_README_KEYNAME: &str = "readme";
pub const ANVILPY_PROJECT_PYTHONV_REQ_KEYNAME: &str = "requires-python";
pub const ANVILPY_PROJECT_CLASSIFIERS_KEYNAME: &str = "classifiers";
pub const ANVILPY_PROJECT_AUTHORS_KEYNAME: &str = "authors";
pub const ANVILPY_PROJECT_URLS_KEYNAME: &str = "urls";
pub const ANVILPY_PROJECT_ENTRYPOINTS_KEYNAME: &str = "scripts";
pub const ANVILPY_BUILD_REQS_KEYNAME: &str = "requires";
pub const ANVILPY_BUILD_BACKEND_KEYNAME: &str = "build-backend";
pub const ANVILPY_UNPACKDIR_NAME: &str = "unpack";

#[derive(Debug)]
pub struct Author {
    /// Author name
    pub name: String,
    /// Author email
    pub email: Option<String>,
}

#[derive(Debug)]
pub struct UrlEntry {
    /// Url name
    pub name: String,
    /// Url anchor
    pub link: Url,
}

#[derive(Debug)]
pub struct ScriptEntry {
    /// Script name
    pub name: String,
    /// Script entrypoint path
    pub entrypoint: String,
}

#[derive(Debug)]
pub struct BuildSystem {
    /// Requirements
    pub reqs: Vec<String>,
    /// Backend
    pub backend: String,
}

#[derive(Debug)]
pub struct AnvilPyEnv {

    /// Project name
    pub proj_name: String,

    /// Project version
    pub version: String,

    /// Authors
    pub authors: Vec<Author>,

    /// Short description
    pub description: String,

    /// Readme file name
    pub readme_path: PathBuf,

    /// Python version requirement
    pub python_version_req: String,

    /// Classifier strings
    pub classifiers: Vec<String>,

    /// Entrypoints
    pub scripts: Vec<ScriptEntry>,

    /// Project links
    pub urls: Vec<UrlEntry>,

    /// Build system
    pub build_sys: BuildSystem,
}

pub fn parse_pyproject_toml(pyproj_path: &PathBuf) -> Result<AnvilPyEnv,String> {
    let start_time = Instant::now();
    let pyproj = fs::read_to_string(pyproj_path).expect("Could not read {pyproj_path} contents");
    //trace!("Pyproject contents: {{{}}}", pyproj);
    let mut pyproj_dir = pyproj_path.clone();
    if ! pyproj_dir.pop() {
        error!("Failed pop for {{{}}}", pyproj_dir.display());
        return Err(format!("Unexpected pyproj_dir value: {{{}}}", pyproj_dir.display()));
    }
    return parse_pyproject_tomlvalue(&pyproj, pyproj_path, start_time);
}

fn parse_pyproject_tomlvalue(pyproj_str: &str, pyproj_path: &PathBuf, start_time: Instant) -> Result<AnvilPyEnv,String> {
    let toml_value = pyproj_str.parse::<Table>();
    match toml_value {
        Ok(y) => {
            let mut anvilpy_env: AnvilPyEnv = AnvilPyEnv {
                proj_name: "".to_string(),
                version: "".to_string(),
                authors: Vec::<Author>::new(),
                description: "".to_string(),
                python_version_req: "".to_string(),
                readme_path: PathBuf::from(""),
                classifiers: Vec::<String>::new(),
                scripts: Vec::<ScriptEntry>::new(),
                urls: Vec::<UrlEntry>::new(),
                build_sys: BuildSystem { reqs: Vec::<String>::new(), backend: "".to_string() },
            };
            trace!("Toml value: {{{}}}", y);
            if let Some(project_table) = y.get("project").and_then(|v| v.as_table()) {
                if let Some(proj_vers) = project_table.get(ANVILPY_PROJECT_VERSION_KEYNAME) {
                    let anvilpy_proj_v_str = proj_vers.as_str().expect("toml conversion failed");
                    debug!("anvilpy_version: {{{anvilpy_proj_v_str}}}");
                    anvilpy_env.version = anvilpy_proj_v_str.to_string();
                    /*
                    if is_semver(anvilpy_proj_v_str) {
                        debug!("anvilpy_version: {{{anvilpy_proj_v_str}}}");
                    } else {
                        error!("anvilpy_version is not a valid semver: {{{}}}", anvilpy_proj_v_str);
                        return Err("Invalid anvilpy_version: {{{anvilpy_proj_v_str}}}, not a semver.".to_string());
                    }
                    */
                } else {
                    debug!("Missing ANVILPY_PROJECT_VERSION definition.");
                }
                if let Some(proj_name) = project_table.get(ANVILPY_PROJECT_NAME_KEYNAME) {
                    let anvilpy_proj_name_str = proj_name.as_str().expect("toml conversion failed");
                    debug!("anvilpy_name: {{{anvilpy_proj_name_str}}}");
                    anvilpy_env.proj_name = anvilpy_proj_name_str.to_string();
                } else {
                    debug!("Missing ANVILPY_PROJECT_NAME definition.");
                }
                if let Some(proj_desc) = project_table.get(ANVILPY_PROJECT_DESC_KEYNAME) {
                    let anvilpy_proj_desc_str = proj_desc.as_str().expect("toml conversion failed");
                    debug!("anvilpy_desc: {{{anvilpy_proj_desc_str}}}");
                    anvilpy_env.description = anvilpy_proj_desc_str.to_string();
                } else {
                    debug!("Missing ANVILPY_PROJECT_DESC definition.");
                }
                if let Some(proj_readme) = project_table.get(ANVILPY_PROJECT_README_KEYNAME) {
                    let anvilpy_proj_readme_str = proj_readme.as_str().expect("toml conversion failed");
                    debug!("anvilpy_readme: {{{anvilpy_proj_readme_str}}}");
                    anvilpy_env.readme_path = PathBuf::from(anvilpy_proj_readme_str.to_string());
                } else {
                    debug!("Missing ANVILPY_PROJECT_DESC definition.");
                }
                if let Some(proj_pyversion_req) = project_table.get(ANVILPY_PROJECT_PYTHONV_REQ_KEYNAME) {
                    let anvilpy_proj_pyversion_req_str = proj_pyversion_req.as_str().expect("toml conversion failed");
                    debug!("anvilpy_python_version_req: {{{anvilpy_proj_pyversion_req_str}}}");
                    anvilpy_env.python_version_req = anvilpy_proj_pyversion_req_str.to_string();
                } else {
                    debug!("Missing ANVILPY_PROJECT_PYVERSION_REQ definition.");
                }
                if let Some(authors_val) = project_table.get(ANVILPY_PROJECT_AUTHORS_KEYNAME) {
                    if let Some(authors_array) = authors_val.as_array() {
                        for author in authors_array {
                            if let Some(author_table) = author.as_table() {
                                let name = author_table.get("name").and_then(|v| v.as_str()).expect("toml conversion failed");
                                let email = author_table.get("email").and_then(|v| v.as_str()).expect("toml conversion failed");
                                let auth = Author { name: name.to_string(), email: Some(email.to_string())};
                                debug!("anvilpy_author: {{{:?}}}", auth);
                                anvilpy_env.authors.push(auth);
                            }
                        }
                    } else {
                        error!("ANVILPY_PROJECT_AUTHORS is not an array");
                        return Err("Authors is not an array".to_string());
                    }
                } else {
                    warn!("Missing ANVILPY_AUTHORS definition.");
                }
                if let Some(classifiers_val) = project_table.get(ANVILPY_PROJECT_CLASSIFIERS_KEYNAME) {
                    if let Some(classifiers_array) = classifiers_val.as_array() {
                        for cls in classifiers_array {
                            let cls_str = cls.as_str().expect("toml conversion failed");
                            debug!("anvilpy_classifier: {{{}}}", cls_str);
                            anvilpy_env.classifiers.push(cls_str.to_string());
                        }
                    } else {
                        error!("ANVILPY_PROJECT_CLASSIFIERS is not an array");
                        return Err("Classifiers is not an array".to_string());
                    }
                } else {
                    warn!("Missing ANVILPY_CLASSIFIERS definition.");
                }
                if let Some(urls_val) = project_table.get(ANVILPY_PROJECT_URLS_KEYNAME) {
                    if let Some(urls_table) = urls_val.as_table() {
                        for (k, v) in urls_table {
                            if let Some(url) = v.as_str() {
                                let url_with_protocol = if url::Url::parse(url).is_ok() {
                                    url.to_string()
                                } else {
                                    trace!("{{{url}}} was not a valid url, trying to prepend https protocol");
                                    format!("https://{}", url)
                                };

                                debug!("anvilpy_project_url: {{{}}}", url_with_protocol);
                                debug!("anvilpy_project_url_name: {{{}}}", k);
                                let url_parsed;
                                match Url::parse(&url_with_protocol) {
                                    Ok(u) => {
                                        url_parsed = u;
                                    }
                                    Err(e) => {
                                        error!("Failed parsing url: {{{}}}. Err was: {e}", url);
                                        return Err("Failed parsing url".to_string());
                                    }
                                }
                                let url_entry = UrlEntry { name: k.to_string(), link: url_parsed };
                                anvilpy_env.urls.push(url_entry);
                            }
                        }
                    } else {
                        error!("ANVILPY_URLS is not a table");
                        return Err("Urls is not an table".to_string());
                    }
                } else {
                    warn!("Missing ANVILPY_URLS definition.");
                }
                if let Some(entrypoints_val) = project_table.get(ANVILPY_PROJECT_ENTRYPOINTS_KEYNAME) {
                    if let Some(entrypoints_table) = entrypoints_val.as_table() {
                        for (k, v) in entrypoints_table {
                            if let Some(entry_path) = v.as_str() {
                                debug!("anvilpy_project_entrypoint_path: {{{}}}", entry_path);
                                debug!("anvilpy_project_entrypoint_name: {{{}}}", k);
                                let script_entry = ScriptEntry { name: k.to_string(), entrypoint: entry_path.to_string()};
                                anvilpy_env.scripts.push(script_entry);
                            }
                        }
                    } else {
                        error!("ANVILPY_ENTRYPOINTS is not a table");
                        return Err("Scripts is not an table".to_string());
                    }
                } else {
                    warn!("Missing ANVILPY_ENTRYPOINTS definition.");
                }
            } else {
                error!("Missing project section.");
                return Err(format!("Missing project section in {{{}}}", pyproj_path.display()));
            }
            if let Some(build_system_table) = y.get("build-system").and_then(|v| v.as_table()) {
                if let Some(reqs) = build_system_table.get(ANVILPY_BUILD_REQS_KEYNAME) {
                    if let Some(reqs_arr) = reqs.as_array() {
                        for item in reqs_arr {
                            match item {
                                toml::Value::String(str_v) => {
                                    trace!("ANVILPY_BUILD_REQS: {{{str_v}}}");
                                    anvilpy_env.build_sys.reqs.push(str_v.to_string());
                                }
                                _ => {
                                    error!("Unexpected item for reqs arrays: {{{}}}", item);
                                    return Err("Unexpected item in reqs array".to_string());
                                }
                            }
                        }
                    } else {
                        error!("ANVILPY_BUILD_REQS is not an array");
                        return Err("Reqs is not an array".to_string());
                    }
                } else {
                    warn!("Missing ANVILPY_BUILD_REQS definition.");
                }
                if let Some(build_backend) = build_system_table.get(ANVILPY_BUILD_BACKEND_KEYNAME) {
                    let backend_str = build_backend.as_str().expect("toml conversion failed");
                    trace!("ANVILPY_BUILD_BACKEND: {{{backend_str}}}");
                    anvilpy_env.build_sys.backend = backend_str.to_string();
                } else {
                    warn!("Missing ANVILPY_BUILD_BACKEND definition.");
                }
            } else {
                error!("Missing ANVILPY_BUILDSYSTEM section.");
                return Err(format!("Missing build-backend section in {{{}}}", pyproj_path.display()));
            }
            let elapsed = start_time.elapsed();
            debug!("Done parsing pyproject.toml. Elapsed: {:.2?}", elapsed);
            return Ok(anvilpy_env);
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done parsing pyproject.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed parsing {{{}}}  as TOML. Err: [{}]", pyproj_str, e);
            return Err("Failed parsing TOML".to_string());
        }
    }
}

pub fn unpack_srcdist(pack_path: &PathBuf) -> Result<PathBuf,String> {
    debug!("TODO: add unpack step");
    let tar_gz = File::open(pack_path);
    match tar_gz {
        Ok(tar_gz) => {
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            let mut outdir = pack_path.clone();
            outdir.pop();
            let unpack_res = archive.unpack(outdir.clone());
            match unpack_res {
                Ok(_) => {
                    debug!("Unpacked srcdist {{{}}} to {{{}}}", pack_path.display(), outdir.display());
                    return Ok(outdir);
                }
                Err(e) => {
                    error!("Failed unpacking srcdist from {{{}}}. Err: {e}", pack_path.display());
                    return Err("Failed unpacking srcdist".to_string());
                }
            }
        }
        Err(e) => {
            error!("Failed opening srcdist pack at {{{}}}. Err: {e}", pack_path.display());
            return Err("Failed opening srcdist".to_string());
        }
    }
}


pub fn gen_anvilpy_shim(shim_path: &PathBuf, module_path: &str, entrypoint_func: &str) -> Result<String,String> {
    trace!("Generating anvilpy shim. Target path: {{{}}} Module path: {{{}}} Function: {{{}}}", shim_path.display(), module_path, entrypoint_func);
    let output = File::create(shim_path);
    let shim_string = format!("#!/bin/python3\n\n##\n# Generated by invil v{INVIL_VERSION}\n# Repo at https://github.com/jgabaut/invil\n##\n\nimport sys\nimport re\nfrom {ANVILPY_UNPACKDIR_NAME}.{module_path} import {entrypoint_func}\n\nif __name__ == \'__main__\':\n    sys.argv[0] = re.sub(r'(-script\\.pyw|\\.exe)?$', '', sys.argv[0])\n    sys.exit({entrypoint_func}())\n");
    match output {
        Ok(mut f) => {
            let res = write!(f, "{}", shim_string);
            match res {
                Ok(_) => {
                    debug!("Done generating shim file");
                }
                Err(e) => {
                    error!("Failed printing shim file");
                    return Err(e.to_string());
                }
            }
            if cfg!(target_os = "windows") {
                todo!("Support setting executable bit for shim on windows");
                /*
                 * let output = Command::new("cmd")
                 *   .args(["/C", "echo hello"])
                 *   .output()
                 *   .expect("failed to execute process")
                 */
            } else {
                let metadata = f.metadata();
                match metadata {
                    Ok(m) => {
                        trace!("Setting mode 744 for shim");
                        let mut permissions = m.permissions();
                        // Add execute permission for the owner
                        permissions.set_mode(permissions.mode() | 0o100);
                        // Set the updated permissions
                        fs::set_permissions(shim_path, permissions).expect("Failed to set file permissions");
                    }
                    Err(e) => {
                        error!("Failed getting metadata for shim. Err: {e}");
                        return Err("Failed getting shim metadata".to_string());
                    }
                }
            }
            return Ok(format!("Generated {{{}}}", shim_path.display()));
        }
        Err(_) => {
            return Err("Failed gen of shim file".to_string());
        }
    }

}

// From https://doc.rust-lang.org/rust-by-example/std_misc/fs.html
// A simple implementation of `% touch path` (ignores existing files)
fn touch_file(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn post_unpack(unpack_initpy_path: &PathBuf, bindir_path: &PathBuf, env: &AmbosoEnv) -> Result<String,String> {
    match touch_file(&unpack_initpy_path) {
        Ok(_) => {},
        Err(e) => {
            error!("Failed touch for {{{}}}. Err: {e}", unpack_initpy_path.display());
            return Err("Failed touch for unpack initpy".to_string());
        }
    }

    debug!("TODO: prep shims for all entrypoints");

    let mut shims_made = 0;
    for entry in &env.anvilpy_env.as_ref().expect("Failed initialising anvilpy_env").scripts {
        let entrypoint_parts: Vec<&str> = entry.entrypoint.splitn(2, ':').collect();
        let module_path = entrypoint_parts[0];
        let entrypoint_funcname = entrypoint_parts[1];
        let shimname = &entry.name;
        debug!("TODO: prep shim {{{}}} -> {{{}}} : {{{}}}", shimname, module_path, entrypoint_funcname);
        let mut shim_path = bindir_path.clone();
        shim_path.push(shimname);
        let shimgen_res = gen_anvilpy_shim(&shim_path, module_path, entrypoint_funcname);
        match shimgen_res {
            Ok(_) => { shims_made += 1; }
            Err(e) => {
                error!("Failed generating shim at {{{}}}", shim_path.display());
                return Err(e);
            }
        }
    }

    Ok(format!("Shims made: {{{shims_made}}}"))
}
