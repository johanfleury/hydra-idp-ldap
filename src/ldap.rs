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

use ldap3::{LdapConn, LdapError, ResultEntry, Scope, SearchEntry};
use serde_json::json;
use serde_json::value::Value;
use std::collections::HashMap;
use structopt::StructOpt;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    LdapError(#[from] LdapError),

    #[error("can’t find user {0}")]
    UserNotFound(String),

    #[error("invalid credentials")]
    InvalidCredentials,
}

#[derive(Debug, StructOpt)]
pub struct Opts {
    #[structopt(
        name = "ldap.url",
        long = "ldap.url",
        env = "LDAP_URL",
        hide_env_values = true,
        value_name = "url",
        help = "URL to the LDAP server (example: ldap://ldap.example.org:389)",
        display_order = 40
    )]
    url: Url,

    #[structopt(
        name = "ldap.bind-dn",
        long = "ldap.bind-dn",
        env = "LDAP_BIND_DN",
        hide_env_values = true,
        value_name = "string",
        help = "LDAP DN to bind to",
        display_order = 41
    )]
    bind_dn: String,

    #[structopt(
        name = "ldap.bind-pw",
        long = "ldap.bind-pw",
        env = "LDAP_BIND_PW",
        hide_env_values = true,
        value_name = "string",
        help = "LDAP bind DN password",
        display_order = 42
    )]
    bind_pw: String,

    #[structopt(
        name = "ldap.users-dn",
        long = "ldap.users-dn",
        env = "LDAP_USERS_DN",
        hide_env_values = true,
        value_name = "string",
        help = "Base DN to search for users",
        display_order = 43
    )]
    users_dn: String,

    #[structopt(
        name = "ldap.users-filter",
        long = "ldap.users-filter",
        env = "LDAP_USERS_FILTER",
        hide_env_values = true,
        value_name = "string",
        default_value = "(&(objectClass=inetOrgPerson)(|(uid={login})(mail={login})))",
        help = "Search filter for users (the special string `{login}` will be replaced by the \
                user’s provided login)",
        display_order = 44
    )]
    users_filter: String,

    #[structopt(
        name = "ldap.groups-dn",
        long = "ldap.groups-dn",
        env = "LDAP_GROUPS_DN",
        hide_env_values = true,
        value_name = "string",
        help = "Base DN to search for groups",
        display_order = 45
    )]
    groups_dn: Option<String>,

    #[structopt(
        name = "ldap.groups-filter",
        long = "ldap.groups-filter",
        env = "LDAP_GROUPS_FILTER",
        hide_env_values = true,
        value_name = "string",
        default_value = "(&(objectClass=groupOfNames)(member={user_dn}))",
        help = "Search filter for groups (the special string `{user_dn}` will be replaced by the \
                user’s DN)",
        display_order = 46
    )]
    groups_filter: String,
}

pub struct LDAP {
    url: Url,
    bind_dn: String,
    bind_pw: String,
    users_dn: String,
    users_filter: String,
    groups_dn: Option<String>,
    groups_filter: String,
}

impl LDAP {
    pub fn new(opts: Opts) -> LDAP {
        LDAP {
            url: opts.url,
            bind_dn: opts.bind_dn,
            bind_pw: opts.bind_pw,
            users_dn: opts.users_dn,
            users_filter: opts.users_filter,
            groups_dn: opts.groups_dn,
            groups_filter: opts.groups_filter,
        }
    }

    pub fn get_user_attrs(
        &self,
        login: &str,
        attrs: Vec<String>,
    ) -> Result<HashMap<String, Value>, Error> {
        let filter: String = self.users_filter.replace("{login}", login);

        let entries = self.search(self.users_dn.as_str(), filter.as_str(), attrs)?;

        if let Some(entry) = entries.first() {
            let entry = SearchEntry::construct(entry.clone());

            let mut h: HashMap<String, Value> = HashMap::new();
            h.insert("dn".to_string(), json!(entry.dn));

            for (attr, values) in entry.attrs {
                let value = match values.len() {
                    1 => values[0].clone(),
                    _ => values.join(","),
                };
                h.insert(attr, json!(value));
            }

            let groups = self.get_user_groups(entry.dn.as_str())?;
            h.insert("groups".to_string(), json!(groups));

            Ok(h)
        } else {
            Err(Error::UserNotFound(login.to_string()))
        }
    }

    pub fn validate_credentials(&self, dn: &str, password: &str) -> Result<bool, Error> {
        match self.authenticate(dn, password) {
            Ok(_) => Ok(true),
            Err(e) => {
                if let Error::InvalidCredentials = e {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn authenticate(&self, dn: &str, password: &str) -> Result<LdapConn, Error> {
        let mut conn = LdapConn::new(self.url.as_str())?;
        let r = conn.simple_bind(dn, password).map_err(Error::LdapError)?;

        // LDAP_INVALID_CREDENTIALS
        if r.rc == 49 {
            Err(Error::InvalidCredentials)
        } else {
            Ok(conn)
        }
    }

    fn get_user_groups(&self, user_dn: &str) -> Result<Vec<String>, Error> {
        let base_dn = match self.groups_dn.clone() {
            Some(dn) => dn,
            None => {
                debug!("Skipping searching for groups as groups search DN is not set");
                return Ok(vec![]);
            }
        };

        let filter: String = self.groups_filter.replace("{user_dn}", user_dn);

        let mut groups: Vec<String> = vec![];

        for entry in self.search(base_dn.as_str(), filter.as_str(), vec!["cn".to_string()])? {
            let entry = SearchEntry::construct(entry.clone());

            for (attr, values) in entry.attrs {
                if attr == "cn" {
                    groups.push(values[0].clone());
                }
            }
        }

        Ok(groups)
    }

    fn search(
        &self,
        base_dn: &str,
        filter: &str,
        attrs: Vec<String>,
    ) -> Result<Vec<ResultEntry>, Error> {
        let mut conn = self.authenticate(self.bind_dn.as_str(), self.bind_pw.as_str())?;

        let (entries, _) = conn
            .search(base_dn, Scope::Subtree, filter, attrs)?
            .success()?;

        Ok(entries)
    }
}
