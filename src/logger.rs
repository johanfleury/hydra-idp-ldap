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

use chrono::Utc;
use log::{Metadata, Record, STATIC_MAX_LEVEL};
use std::path::Path;

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= STATIC_MAX_LEVEL
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if record.file().is_some() && record.line().is_some() {
            println!(
                "{} - {}#{} - {} - {}",
                Utc::now(),
                Path::new(record.module_path().unwrap())
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                record.line().unwrap(),
                record.level(),
                record.args()
            );
        } else {
            println!("{} - {} - {}", Utc::now(), record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
