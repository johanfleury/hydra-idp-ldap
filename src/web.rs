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

use anyhow::Result;
use hydra_client::Hydra;
use rocket::config::{Config, Environment};
use rocket::http::Status;
use rocket::request::Form;
use rocket::response::Redirect;
use rocket::{Request, State};
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use structopt::StructOpt;

use crate::ldap::LDAP;
use crate::parse;

const STATIC_DIR: &str = "static/";
const TEMPLATE_DIR: &str = "templates/";

#[derive(Debug, StructOpt)]
pub struct Opts {
    #[structopt(
        name = "web.listen-address",
        long = "web.listen-address",
        env = "WEB_LISTEN_ADDRESS",
        hide_env_values = true,
        value_name = "address",
        parse(try_from_str = parse::sock_addr),
        default_value = "0.0.0.0:8080",
        help = "Address to listen on (in the form <ip>:<port>)",
        display_order = 20,
    )]
    listen_address: SocketAddr,

    #[structopt(
        name = "web.tls-cert-file",
        long = "web.tls-cert-file",
        env = "WEB_TLS_CERT_FILE",
        hide_env_values = true,
        value_name = "file",
        requires = "web.tls-key-file",
        help = "Path to a certificate chain file in PEM format (enables TLS)",
        parse(try_from_str = parse::file),
        display_order = 21,
    )]
    tls_cert_file: Option<String>,

    #[structopt(
        name = "web.tls-key-file",
        long = "web.tls-key-file",
        env = "WEB_TLS_KEY_FILE",
        hide_env_values = true,
        value_name = "file",
        parse(try_from_str = parse::file),
        requires = "web.tls-cert-file",
        help = "Path to a private key file in PEM format (enables TLS)",
        display_order = 22,
    )]
    tls_key_file: Option<String>,

    #[structopt(
        name = "web.base-path",
        long = "web.base-path",
        env = "WEB_BASE_PATH",
        hide_env_values = true,
        value_name = "string",
        parse(try_from_str = parse::path),
        default_value = "/",
        help = "Path prefix for endpoints",
        display_order = 23,
    )]
    base_path: String,

    #[structopt(flatten)]
    oauth: OauthOpts,
}

#[derive(Debug, StructOpt)]
pub struct OauthOpts {
    #[structopt(
        name = "oauth.login-remember-for",
        long = "oauth.login-remember-for",
        env = "OAUTH_LOGIN_REMEMBER_FOR",
        hide_env_values = true,
        value_name = "integer",
        default_value = "0",
        help = "Time in seconds defining how long a sucessful login should be remembered (0 means \
                it will be until browser tab or window is closed).",
        display_order = 50
    )]
    login_remember_for: u64,

    #[structopt(
        name = "oauth.attrs-map",
        long = "oauth.attrs-map",
        env = "OAUTH_ATTRS_MAP",
        hide_env_values = true,
        value_name = "map",
        parse(try_from_str = parse::comma_separated_key_value),
        default_value = "cn:name,sn:family_name,givenName:given_name,mail:email",
        help = "A list of comma separated <LDAP attribute name>:<OAuth claim name>",
        display_order = 51,
    )]
    attrs_map: HashMap<String, String>,

    #[structopt(
        name = "oauth.claims-map",
        long = "oauth.claims-map",
        env = "OAUTH_CLAIMS_MAP",
        hide_env_values = true,
        value_name = "map",
        parse(try_from_str = parse::comma_separated_key_value),
        default_value = "name:profile,family_name:profile,given_name:profile,email:email",
        help = "A list of comma separated <OAuth claim name>:<OAuth scope name>",
        display_order = 52,
    )]
    claims_map: HashMap<String, String>,
}

pub fn launch(opts: Opts, hydra: Hydra, ldap: LDAP) -> Result<()> {
    let config_builder = Config::build(Environment::Production)
        .address(opts.listen_address.ip().to_string())
        .port(opts.listen_address.port())
        .extra("template_dir", TEMPLATE_DIR);

    let config_builder = match opts.tls_cert_file.is_some() && opts.tls_key_file.is_some() {
        true => config_builder.tls(opts.tls_cert_file.unwrap(), opts.tls_key_file.unwrap()),
        false => config_builder,
    };

    let config = match config_builder.finalize() {
        Ok(config) => config,
        Err(_) => {
            // This is the only possible cause of error
            return Err(anyhow!("Unable to read TLS certificate or private key."));
        }
    };

    let static_path = Path::new(opts.base_path.as_str())
        .join("/static/")
        .to_str()
        .unwrap()
        .to_string();

    let rocket = rocket::custom(config)
        .mount(
            opts.base_path.as_str(),
            routes![login, post_login, consent, logout, post_logout, error,],
        )
        .mount(static_path.as_str(), StaticFiles::from(STATIC_DIR))
        .register(catchers![not_found, internal_server_error])
        .manage(opts.oauth)
        .manage(hydra)
        .manage(ldap)
        .attach(Template::fairing());

    // rocket.launch only exit on error
    Err(anyhow!(rocket.launch()))
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Responder)]
enum Response {
    Template(Template),
    Redirect(Redirect),
    Status(Status),
}

#[derive(FromForm)]
struct LoginForm {
    login: String,
    password: String,
    remember: Option<bool>,
}

fn render_login_template(form_error: Option<String>) -> Template {
    let mut context: HashMap<String, String> = HashMap::new();

    if let Some(form_error) = form_error {
        context.insert("form_error".to_string(), form_error);
    }

    Template::render("login", &context)
}

#[get("/login?<login_challenge>")]
fn login(login_challenge: String, hydra: State<Hydra>) -> Response {
    let hydra = hydra.clone();

    if login_challenge.is_empty() {
        return Response::Status(Status::NotFound);
    }

    let r = match hydra.get_login_request(login_challenge.clone()) {
        Ok(r) => r,
        Err(e) => {
            warn!("unable to get login request details: {}", e);
            return Response::Status(Status::InternalServerError);
        }
    };

    if r.skip {
        return match hydra.accept_login_request(login_challenge, r.subject, None, None, None, None)
        {
            Ok(r) => Response::Redirect(Redirect::to(r.redirect_to)),
            Err(e) => {
                warn!("unable to accept login request: {}", e);
                Response::Status(Status::InternalServerError)
            }
        };
    }

    Response::Template(render_login_template(None))
}

#[post("/login?<login_challenge>", data = "<form>")]
fn post_login(
    login_challenge: String,
    form: Form<LoginForm>,
    oauth_opts: State<OauthOpts>,
    hydra: State<Hydra>,
    ldap: State<LDAP>,
) -> Response {
    if login_challenge.is_empty() {
        return Response::Status(Status::NotFound);
    }

    let attrs = match ldap.get_user_attrs(
        form.login.as_str(),
        oauth_opts.attrs_map.keys().cloned().collect(),
    ) {
        Ok(attrs) => attrs,
        Err(e) => {
            warn!("Unable to find user in LDAP database: {}", e);
            return Response::Template(render_login_template(Some(
                "Invalid login or password.".to_string(),
            )));
        }
    };

    match ldap.validate_credentials(attrs["dn"].as_str(), form.password.as_str()) {
        Ok(ok) => {
            if !ok {
                info!("Invalid login or password for {}", form.login);
                return Response::Template(render_login_template(Some(
                    "Invalid login or password.".to_string(),
                )));
            }
        }
        Err(e) => {
            warn!("LDAP Error: {}", e);
            return Response::Status(Status::InternalServerError);
        }
    };

    match hydra.accept_login_request(
        login_challenge.clone(),
        form.login.clone(),
        None,
        None,
        form.remember,
        Some(oauth_opts.login_remember_for),
    ) {
        Ok(r) => {
            info!(
                "accepted login request with challenge `{}` for `{}`",
                login_challenge, form.login
            );
            Response::Redirect(Redirect::to(r.redirect_to))
        }
        Err(e) => {
            warn!("unable to accept login request: {}", e);
            Response::Status(Status::InternalServerError)
        }
    }
}

#[get("/consent?<consent_challenge>")]
fn consent(
    consent_challenge: String,
    oauth_opts: State<OauthOpts>,
    hydra: State<Hydra>,
    ldap: State<LDAP>,
) -> Response {
    let hydra = hydra.clone();

    if consent_challenge.is_empty() {
        return Response::Status(Status::NotFound);
    }

    let r = match hydra.get_consent_request(consent_challenge.clone()) {
        Ok(r) => r,
        Err(e) => {
            warn!("Unable to get consent request details: {}", e);
            return Response::Status(Status::InternalServerError);
        }
    };

    let mut claims: HashMap<String, String> = HashMap::new();

    let attrs = match ldap.get_user_attrs(
        r.subject.as_str(),
        oauth_opts.attrs_map.keys().cloned().collect(),
    ) {
        Ok(attrs) => attrs,
        Err(e) => {
            warn!("Unable to find user in LDAP database: {}", e);
            return Response::Template(render_login_template(Some(
                "Invalid login or password.".to_string(),
            )));
        }
    };

    for (attr_name, attr_value) in attrs {
        let claim_name = match oauth_opts.attrs_map.get(&attr_name) {
            Some(claim_name) => claim_name,
            None => {
                debug!("skiping attribute '{}' not mapped to a claim", attr_name);
                continue;
            }
        };

        let claim_scope = match oauth_opts.claims_map.get(claim_name) {
            Some(claim_scope) => claim_scope,
            None => {
                debug!("skiping claim '{}' not mapped to a scope", claim_name);
                continue;
            }
        };

        if !r.requested_scope.contains(claim_scope) {
            debug!(
                "skiping claim '{}' as client didn’t request scope '{}'",
                claim_name, claim_scope
            )
        }

        claims.insert(claim_name.to_string(), attr_value.to_string());
    }

    match hydra.accept_consent_request(
        consent_challenge,
        r.requested_access_token_audience,
        r.requested_scope,
        Some(true),
        Some(0), // Remember consent request indefinitely
        Some(claims),
    ) {
        Ok(r) => Response::Redirect(Redirect::to(r.redirect_to)),
        Err(e) => {
            warn!("unable to accept consent request: {}", e);
            Response::Status(Status::InternalServerError)
        }
    }
}

#[get("/logout?<logout_challenge>")]
fn logout(logout_challenge: String, hydra: State<Hydra>) -> Response {
    if logout_challenge.is_empty() {
        return Response::Status(Status::NotFound);
    }

    match hydra.accept_logout_request(logout_challenge.clone()) {
        Ok(r) => {
            info!(
                "accepted logout request with challenge `{}`",
                logout_challenge
            );
            Response::Redirect(Redirect::to(r.redirect_to))
        }
        Err(e) => {
            warn!("unable to accept login request: {}", e);
            Response::Status(Status::InternalServerError)
        }
    }
}

#[get("/post-logout")]
fn post_logout() -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("post-logout", &context)
}

#[get("/error?<error>&<error_description>&<error_hint>")]
fn error(error: String, error_description: String, error_hint: String) -> Template {
    let mut context: HashMap<String, String> = HashMap::new();
    context.insert("name".to_string(), error);
    context.insert("description".to_string(), error_description);
    context.insert("hint".to_string(), error_hint);

    Template::render("error", &context)
}

#[catch(404)]
fn not_found(_req: &Request) -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("404", &context)
}

#[catch(500)]
fn internal_server_error(_req: &Request) -> Template {
    let context: HashMap<String, String> = HashMap::new();
    Template::render("500", &context)
}
