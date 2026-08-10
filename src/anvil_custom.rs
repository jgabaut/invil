//  SPDX-License-Identifier: GPL-3.0-only
/*  Build tool with support for git tags, wrapping make.
 *  Copyright (C) 2023-2026  jgabaut
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
use std::path::PathBuf;
use std::time::Instant;
use std::cmp::Ordering;
use std::fs;
use toml::Table;
use regex::Regex;
use crate::core::{SemVerKey, semver_compare, MIN_AMBOSO_V_CUST_RECIPES};

pub const ANVILCUST_CUSTOM_BUILDER_KEYNAME: &str = "custombuilder";
pub const ANVILCUST_RECIPES_KEYNAME: &str = "recipe";
pub const ANVILCUST_RECIPE_CONF_KEYNAME: &str = "conf";
pub const ANVILCUST_RECIPE_BUILD_KEYNAME: &str = "build";
pub const ANVILCUST_RECIPE_VERS_KEYNAME: &str = "vers";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnvilRecipe {
    pub vers: SemVerKey,
    pub conf: Option<String>,
    pub build: String,
}

impl Ord for AnvilRecipe {
    fn cmp(&self, other: &Self) -> Ordering {
        semver_compare(&self.vers.0, &other.vers.0)
    }
}

impl PartialOrd for AnvilRecipe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct AnvilCustomEnv {
    /// Custom builder command string
    pub custom_builder: String,
    pub recipes: Vec<AnvilRecipe>,
}

pub fn parse_anvilcustom_toml(stego_path: &PathBuf, anvil_version: &str) -> Result<AnvilCustomEnv,String> {
    let start_time = Instant::now();
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    //trace!("Pyproject contents: {{{}}}", pyproj);
    let mut stego_dir = stego_path.clone();
    if ! stego_dir.pop() {
        error!("Failed pop for {{{}}}", stego_dir.display());
        return Err(format!("Unexpected stego_dir value: {{{}}}", stego_dir.display()));
    }
    return parse_anvilcustom_tomlvalue(&stego, stego_path, anvil_version, start_time);
}

fn has_reserved_char(input: &str) -> bool {
    // Use a delimited raw string to handle backslashes correctly
    let re = Regex::new(r#"[\$\`\";\|\&><\*\(\)\#\{\}\[\]]"#).unwrap();

    // Check if any forbidden character is found
    if re.is_match(input) {
        return true;
    }

    false
}

fn parse_anvilcustom_tomlvalue(stego_str: &str, stego_path: &PathBuf, anvil_version: &str, start_time: Instant) -> Result<AnvilCustomEnv,String> {
    let toml_value = stego_str.parse::<Table>();
    match toml_value {
        Ok(y) => {
            let mut anvilcustom_env: AnvilCustomEnv = AnvilCustomEnv {
                custom_builder : "".to_string(),
                recipes: Vec::new(),
            };
            trace!("Toml value: {{{}}}", y);
            if let Some(anvil_table) = y.get("anvil").and_then(|v| v.as_table()) {
                let mut skip_recipes_parse = false;
                match semver_compare(anvil_version, MIN_AMBOSO_V_CUST_RECIPES) {
                    Ordering::Less => {
                        warn!("Strict behaviour for v{}, skipping reading custom recipes from stego.lock", anvil_version);
                        skip_recipes_parse = true;
                    }
                    Ordering::Equal | Ordering::Greater => {}
                }
                if !skip_recipes_parse {
                    if let Some(recipes) = anvil_table.get(ANVILCUST_RECIPES_KEYNAME) {
                        debug!("anvil_recipe: {{{recipes}}}");
                        if recipes.is_array() {
                            for (i, inner_v) in recipes.as_array().expect("Failed parsing array").iter().enumerate() {
                                if inner_v.is_table() {
                                    let mut recipe = AnvilRecipe {
                                        conf: None,
                                        vers: SemVerKey("".to_string()),
                                        build: "".to_string(),
                                    };
                                    let recipe_tab = inner_v.as_table().expect("Failed parsing table");
                                    if let Some(conf) = recipe_tab.get(ANVILCUST_RECIPE_CONF_KEYNAME) {
                                        let conf_str = conf.to_string();
                                        recipe.conf = Some(conf_str.trim_matches('"').to_string());
                                    }
                                    if let Some(build) = recipe_tab.get(ANVILCUST_RECIPE_BUILD_KEYNAME) {
                                        let build_str = build.to_string();
                                        recipe.build = build_str.trim_matches('"').to_string();
                                    } else {
                                        error!("Missing anvil_recipe[{}]_build", i);
                                        return Err(format!("Missing anvil_recipe[{}]_build definition", i));
                                    }
                                    if let Some(vers) = recipe_tab.get(ANVILCUST_RECIPE_VERS_KEYNAME) {
                                        recipe.vers = SemVerKey(vers.to_string().trim_matches('"').to_string());
                                    } else {
                                        error!("Missing anvil_recipe[{}]_vers", i);
                                        return Err(format!("Missing anvil_recipe[{}]_vers definition", i));
                                    }
                                    anvilcustom_env.recipes.push(recipe);
                                } else {
                                    error!("anvil_recipe[{}] definition is not a table.", i);
                                    return Err(format!("Wrong definition for anvil_recipe[{}] in {{{}}}", i, stego_path.display()));
                                }
                            }
                        } else {
                            error!("ANVILCUST_RECIPES definition is not an array.");
                            return Err(format!("Wrong anvil_recipe definition in {{{}}}", stego_path.display()));
                        }
                    } else {
                        error!("Missing ANVILCUST_RECIPES definition.");
                        return Err(format!("Missing anvil_recipe in {{{}}}", stego_path.display()));
                    }
                } else {
                    if let Some(custom_builder) = anvil_table.get(ANVILCUST_CUSTOM_BUILDER_KEYNAME) {
                        let anvilcust_builder_str = custom_builder.as_str().expect("toml conversion failed");
                        if has_reserved_char(anvilcust_builder_str) {
                            //TODO: warning about the reserved chars?
                            error!("anvil_custombuilder: --> {{{anvilcust_builder_str}}}");
                            return Err("Invalid custombuilder arg".to_string());
                        }
                        debug!("anvil_custombuilder: {{{anvilcust_builder_str}}}");
                        anvilcustom_env.custom_builder = anvilcust_builder_str.to_string();
                    } else {
                        error!("Missing ANVILCUST_CUSTOM_BUILDER definition.");
                        return Err(format!("Missing anvil_custombuilder in {{{}}}", stego_path.display()));
                    }
                }
            } else {
                error!("Missing anvil section.");
                return Err(format!("Missing anvil section in {{{}}}", stego_path.display()));
            }

            let elapsed = start_time.elapsed();
            debug!("Done parsing pyproject.toml. Elapsed: {:.2?}", elapsed);
            return Ok(anvilcustom_env);
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            debug!("Done parsing stego.toml. Elapsed: {:.2?}", elapsed);
            error!("Failed parsing {{{}}}  as TOML. Err: [{}]", stego_str, e);
            return Err("Failed parsing TOML".to_string());
        }
    }
}

pub fn sort_anvilcustom_env(env: &mut AnvilCustomEnv) {
    env.recipes.sort();
}

pub fn find_anvilcustom_recipe(env: &AnvilCustomEnv, query: &str) -> Option<AnvilRecipe> {
    for recipe in &env.recipes {
        match semver_compare(&recipe.vers.0, query) {
            Ordering::Less => continue,
            _ => return Some(recipe.clone()),
        };
    }
    None
}
