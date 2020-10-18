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

#![feature(decl_macro)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate rocket;

mod ldap;
mod logger;
mod parse;
mod web;

use anyhow::{Context, Result};
use hydra_client::Hydra;
use structopt::StructOpt;
use url::Url;

use crate::ldap::LDAP;
use crate::logger::Logger;

#[derive(Debug, StructOpt)]
#[structopt(set_term_width = 0)]
struct Opts {
    #[structopt(
        name = "log.level",
        long = "log.level",
        env = "LOG_LEVEL",
        hide_env_values = true,
        value_name = "string",
        possible_values = &["off", "error", "warn", "info", "debug", "trace"],
        case_insensitive = true,
        default_value = "info",
        help = "Log level",
        display_order = 10,
    )]
    log_level: log::LevelFilter,

    #[structopt(flatten)]
    web: web::Opts,

    #[structopt(
        name = "hydra.url",
        long = "hydra.url",
        env = "HYDRA_URL",
        hide_env_values = true,
        value_name = "url",
        help = "URL of the Hydra admin server",
        display_order = 30
    )]
    hydra_url: Url,

    #[structopt(flatten)]
    ldap: ldap::Opts,
}

static LOGGER: Logger = Logger;

fn main() -> Result<()> {
    let opts: Opts = Opts::from_args();

    log::set_logger(&LOGGER).context("unable to setup logger")?;
    log::set_max_level(opts.log_level);

    debug!("Parsed arguments: {:?}", opts);

    let hydra: Hydra = Hydra::new(opts.hydra_url);
    let ldap: LDAP = LDAP::new(opts.ldap);

    web::launch(opts.web, hydra, ldap).context("Web server failed to start")
}
