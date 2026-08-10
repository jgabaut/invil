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

#[derive(Debug)]
pub struct AnvilRecipe {
    pub vers: SemVerKey,
    pub conf: Option<String>,
    pub build: String,
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
                        todo!("Implement this");
                    } else {
                        error!("Missing ANVILCUST_RECIPES definition.");
                        return Err(format!("Missing anvil_recipe in {{{}}}", stego_path.display()));
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

