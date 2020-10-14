// Copyright 2020 Johan Fleury <jfleury@arcaik.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

pub fn sock_addr(value: &str) -> Result<SocketAddr, String> {
    SocketAddr::from_str(value)
        .map_err(|_| format!("can't parse IP address and/or port from '{}'", value))
}

pub fn file(value: &str) -> Result<String, String> {
    let file = Path::new(value);

    if !file.exists() {
        return Err(format!("no such file or directory: '{}'", value));
    }

    if !file.is_file() {
        return Err(format!("not a file: {}", value));
    }

    Ok(value.to_string())
}

pub fn path(value: &str) -> Result<String, String> {
    if value.starts_with('/') {
        Ok(value.to_string())
    } else {
        Err("path must start with `/`".to_string())
    }
}

pub fn key_value(value: &str) -> Result<(String, String), String> {
    let pos = value
        .find(':')
        .ok_or(format!("invalid key:val format in: {}", value))?;

    Ok((value[..pos].to_string(), value[pos + 1..].to_string()))
}

pub fn comma_separated_key_value(value: &str) -> Result<HashMap<String, String>, String> {
    let mut h: HashMap<String, String> = HashMap::new();

    for item in value.split(',') {
        if item.is_empty() {
            continue;
        }

        match key_value(item) {
            Ok((key, val)) => h.insert(key, val),
            Err(e) => return Err(e),
        };
    }

    Ok(h)
}
