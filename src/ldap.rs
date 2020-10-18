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

use ldap3::{LdapConn, LdapError, Scope, SearchEntry};
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
        name = "ldap.base-dn",
        long = "ldap.base-dn",
        env = "LDAP_BASE_DN",
        hide_env_values = true,
        value_name = "string",
        help = "Base DN to search for users",
        display_order = 43
    )]
    base_dn: String,

    #[structopt(
        name = "ldap.user-filter",
        long = "ldap.user-filter",
        env = "LDAP_USER_FILTER",
        hide_env_values = true,
        value_name = "string",
        default_value = "(&(objectClass=inetOrgPerson)(|(uid={login})(mail={login})))",
        help = "Default search filter for user (the special string `{login}` will be replaced by \
                the user’s provided login)",
        display_order = 44
    )]
    user_filter: String,
}

pub struct LDAP {
    url: Url,
    bind_dn: String,
    bind_pw: String,
    base_dn: String,
    user_filter: String,
}

impl LDAP {
    pub fn new(opts: Opts) -> LDAP {
        LDAP {
            url: opts.url,
            bind_dn: opts.bind_dn,
            bind_pw: opts.bind_pw,
            base_dn: opts.base_dn,
            user_filter: opts.user_filter,
        }
    }

    pub fn get_user_attrs(
        &self,
        login: &str,
        attrs: Vec<String>,
    ) -> Result<HashMap<String, String>, Error> {
        let mut conn = self.authenticate(self.bind_dn.as_str(), self.bind_pw.as_str())?;

        let user_filter: String = self.user_filter.replace("{login}", login);

        let (entries, _) = conn
            .search(
                self.base_dn.as_str(),
                Scope::Subtree,
                user_filter.as_str(),
                attrs,
            )?
            .success()?;

        if let Some(entry) = entries.first() {
            let entry = SearchEntry::construct(entry.clone());

            let mut h: HashMap<String, String> = HashMap::new();
            h.insert("dn".to_string(), entry.dn);

            for (attr, values) in entry.attrs {
                let value = match values.len() {
                    1 => values[0].clone(),
                    _ => values.join(","),
                };
                h.insert(attr, value);
            }
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
}
