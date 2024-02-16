use url::{Url};

use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use toml::Table;

pub const ANVILPY_PROJECT_VERSION_KEYNAME: &str = "version";
pub const ANVILPY_PROJECT_NAME_KEYNAME: &str = "name";
pub const ANVILPY_PROJECT_DESC_KEYNAME: &str = "description";
pub const ANVILPY_PROJECT_README_KEYNAME: &str = "readme";
pub const ANVILPY_PROJECT_PYTHONV_REQ_KEYNAME: &str = "requires-python";
pub const ANVILPY_BUILD_REQS_KEYNAME: &str = "requires";
pub const ANVILPY_BUILD_BACKEND_KEYNAME: &str = "build-backend";

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
    pub scripts: Option<Vec<String>>,

    /// Project links
    pub urls: Option<Vec<UrlEntry>>,

    /// Build system
    pub build_sys: BuildSystem,
}

pub fn parse_pyproject_toml(pyproj_path: &PathBuf) -> Result<AnvilPyEnv,String> {
    let start_time = Instant::now();
    let pyproj = fs::read_to_string(pyproj_path).expect("Could not read {pyproj_path} contents");
    //trace!("Pyproject contents: {{{}}}", pyproj);
    let toml_value = pyproj.parse::<Table>();
    let mut pyproj_dir = pyproj_path.clone();
    if ! pyproj_dir.pop() {
        error!("Failed pop for {{{}}}", pyproj_dir.display());
        return Err(format!("Unexpected pyproj_dir value: {{{}}}", pyproj_dir.display()));
    }
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
                scripts: Some(Vec::<String>::new()),
                urls: Some(Vec::<UrlEntry>::new()),
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
            } else {
                error!("Missing project section.");
                return Err(format!("Missing project section in {{{}}}", pyproj_path.display()));
            }
            if let Some(build_system_table) = y.get("build-system").and_then(|v| v.as_table()) {
                if let Some(reqs) = build_system_table.get(ANVILPY_BUILD_REQS_KEYNAME) {
                    debug!("TODO: parse reqs array");
                    trace!("ANVILPY_BUILD_REQS: {{{reqs}}}");
                    //anvilpy_env.ba = Some(format!("{}", source_name.as_str().expect("toml conversion failed")));
                } else {
                    warn!("Missing ANVILPY_BUILD_REQS definition.");
                }
                if let Some(build_backend) = build_system_table.get(ANVILPY_BUILD_BACKEND_KEYNAME) {
                    trace!("ANVILPY_BUILD_BACKEND: {{{build_backend}}}");
                    //anvil_env.bin = Some(format!("{}", binary_name.as_str().expect("toml conversion failed")));
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
            error!("Failed parsing {{{}}}  as TOML. Err: [{}]", pyproj, e);
            return Err("Failed parsing TOML".to_string());
        }
    }
}
