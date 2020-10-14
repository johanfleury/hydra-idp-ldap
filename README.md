# hydra-idp-ldap

`hydra-idp-ldap` is an LDAP powered, Identity Provider (IdP) for [ORY Hydra
API](https://www.ory.sh/hydra/) written in Rust.

## Project maturity and alternatives

This project is my first real Rust project and I use it for my personal
needs only. It has not been tested at scale.

If you’re looking for a more mature project, please have a look at
[werther](https://github.com/i-core/werther/) from which I took a lot of
inspiration.

## Installation

The easiest (and recommended) way of deploying `hydra-idp-ldap` is with Docker:

```
$ docker run --rm -it --read-only -u 65534:65534 -p 8080:8080 arcaik/hydra-idp-ldap ...
```

## Usage

```
hydra-idp-ldap 0.1.0

USAGE:
    hydra-idp-ldap [OPTIONS] --ldap.base-dn <base-dn> --ldap.bind-dn <bind-dn> --ldap.bind-pw <bind-pw> --hydra.url <hydra-url> --ldap.url <url>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --log.level <log-level>                            Log level [env: LOG_LEVEL]  [default: info]  [possible values: off, error, warn, info, debug, trace]
        --web.listen-address <listen-address>              Address to listen on (in the form <ip>:<port>) [env: WEB_LISTEN_ADDRESS]  [default: 0.0.0.0:8080]
        --web.tls-cert-file <tls-cert-file>                Path to a certificate chain file in PEM format (enables TLS) [env: WEB_TLS_CERT_FILE]
        --web.tls-key-file <web.tls-key-file>              Path to a private key file in PEM format (enables TLS) [env: WEB_TLS_KEY_FILE]
        --web.base-path <base-path>                        Path prefix for endpoints [env: WEB_BASE_PATH]  [default: /]
        --hydra.url <hydra-url>                            URL of the Hydra admin server [env: HYDRA_URL]
        --ldap.url <url>                                   URL to the LDAP server (example: ldap://ldap.example.org:389) [env: LDAP_URL]
        --ldap.bind-dn <bind-dn>                           LDAP DN to bind to [env: LDAP_BIND_DN]
        --ldap.bind-pw <bind-pw>                           LDAP bind DN password [env: LDAP_BIND_PW]
        --ldap.base-dn <base-dn>                           Base DN to search for users [env: LDAP_BASE_DN]
        --ldap.user-filter <user-filter>                   Default search filter for user (the special string `{login}` will be replaced by the user’s provided login) [env: LDAP_USER_FILTER]  [default: (&(objectClass=inetOrgPerson)(|(uid={login})(mail={login})))]
        --oauth.login-remember-for <login-remember-for>    Time in seconds defining how long a sucessful login should be remembered (0 means it will be until browser tab or window is closed). [env: OAUTH_LOGIN_REMEMBER_FOR]  [default: 0]
        --oauth.attrs-map <attrs-map>                      A list of comma separated <LDAP attribute name>:<OAuth claim name> [env: OAUTH_ATTRS_MAP]  [default: cn:name,sn:family_name,givenName:given_name,mail:email]
        --oauth.claims-map <claims-map>                    A list of comma separated <OAuth claim name>:<OAuth scope name> [env: OAUTH_CLAIMS_MAP]  [default: name:profile,family_name:profile,given_name:profile,email:email]
```

### Configuring Hydra

To setup Hydra for usage with `hydra-idp-ldap`, you must set the following
settings accordingly in Hydra’s `config.yaml` (or the equivalent environment
variables):

```
urls:
  login: https://hydra-idp-ldap/login
  consent: https://hydra-idp-ldap/consent
  logout: https://hydra-idp-ldap/logout
  error: https://hydra-idp-ldap/error
  post_logout_redirect: https://hydra-idp-ldap/post-logout
```

## Contributing

This project is [Free Software](LICENCE.md) and every contributions are
welcome.

Please note that this project is released with a [Contributor Code of
Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to
abide by its terms.
