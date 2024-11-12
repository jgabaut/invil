//  SPDX-License-Identifier: GPL-3.0-only
/*  Build tool with support for git tags, wrapping make.
 *  Copyright (C) 2023-2024  jgabaut
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
use std::fs;
use toml::Table;

pub const ANVILCUST_CUSTOM_BUILDER_KEYNAME: &str = "custombuilder";

#[derive(Debug)]
pub struct AnvilCustomEnv {
    /// Custom builder command string
    pub custom_builder: String,
}

pub fn parse_anvilcustom_toml(stego_path: &PathBuf) -> Result<AnvilCustomEnv,String> {
    let start_time = Instant::now();
    let stego = fs::read_to_string(stego_path).expect("Could not read {stego_path} contents");
    //trace!("Pyproject contents: {{{}}}", pyproj);
    let mut stego_dir = stego_path.clone();
    if ! stego_dir.pop() {
        error!("Failed pop for {{{}}}", stego_dir.display());
        return Err(format!("Unexpected stego_dir value: {{{}}}", stego_dir.display()));
    }
    return parse_anvilcustom_tomlvalue(&stego, stego_path, start_time);
}

fn parse_anvilcustom_tomlvalue(stego_str: &str, stego_path: &PathBuf, start_time: Instant) -> Result<AnvilCustomEnv,String> {
    let toml_value = stego_str.parse::<Table>();
    match toml_value {
        Ok(y) => {
            let mut anvilcustom_env: AnvilCustomEnv = AnvilCustomEnv {
                custom_builder : "".to_string(),
            };
            trace!("Toml value: {{{}}}", y);
            if let Some(anvil_table) = y.get("anvil").and_then(|v| v.as_table()) {
                if let Some(custom_builder) = anvil_table.get(ANVILCUST_CUSTOM_BUILDER_KEYNAME) {
                    let anvilcust_builder_str = custom_builder.as_str().expect("toml conversion failed");
                    debug!("anvil_custombuilder: {{{anvilcust_builder_str}}}");
                    anvilcustom_env.custom_builder = anvilcust_builder_str.to_string();
                } else {
                    error!("Missing ANVILCUST_CUSTOM_BUILDER definition.");
                    return Err(format!("Missing anvil_custombuilder in {{{}}}", stego_path.display()));
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

