use structopt::StructOpt;
use super::*;

const HOST_ENV:&'static str = "WORMHOLE_HOST";
const PORT_ENV:&'static str = "WORMHOLE_PORT";
const TLS_OFF_ENV:&'static str = "TLS_OFF";

const DEFAULT_HOST:&'static str = "tunnelto.dev";
const DEFAULT_CONTROL_HOST:&'static str = "wormhole.tunnelto.dev";
const DEFAULT_CONTROL_PORT:&'static str = "10001";

const SETTINGS_DIR:&'static str = ".tunnelto";
const SECRET_KEY_FILE:&'static str = "key.token";

/// Command line arguments
#[derive(Debug, StructOpt)]
#[structopt(name = "tunnelto", author="Alex Grinman <alex@tunnelto.dev>", about = "Expose your local web server to the internet with a public url.")]
struct Opts {
    /// A level of verbosity, and can be used multiple times
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,

    #[structopt(subcommand)]
    command: Option<SubCommand>,

    /// Sets an API authentication key to use for this wormhole
    #[structopt(short = "k", long = "key")]
    key: Option<String>,

    /// Specify a sub-domain for this wormhole
    #[structopt(short = "s", long = "subdomain")]
    sub_domain: Option<String>,

    /// Sets the port to forward incoming tunnel traffic to on localhost
    #[structopt(short = "p", long = "port", default_value = "8000")]
    port: String,

}

#[derive(Debug, StructOpt)]
enum SubCommand {
    /// Store the API Authentication key
    SetAuth {
        /// Sets an API authentication key on disk for future use
        #[structopt(short = "k", long = "key")]
        key: String
    },
}

/// Config
#[derive(Debug, Clone)]
pub struct Config {
    pub client_id: ClientId,
    pub control_url: String,
    pub host: String,
    pub local_port: String,
    pub sub_domain: Option<String>,
    pub secret_key: Option<SecretKey>,
    pub tls_off: bool,
    pub first_run: bool,
}

impl Config {
    /// Parse the URL to use to connect to the wormhole control server
    pub fn get() -> Result<Config, ()> {
        // parse the opts
        let opts: Opts = Opts::from_args();

        if opts.verbose {
            std::env::set_var("RUST_LOG", "tunnelto=debug");
        } else {
            std::env::set_var("RUST_LOG", "tunnelto=error");
        }

        pretty_env_logger::init();

        let (secret_key, sub_domain, local_port) = match opts.command {
            Some(SubCommand::SetAuth { key }) => {
                let key = opts.key.unwrap_or(key);
                let wormhole_dir = match dirs::home_dir().map(|h| h.join(SETTINGS_DIR)) {
                    Some(path) => path,
                    None => {
                        panic!("Could not find home directory to store token.")
                    }
                };
                std::fs::create_dir_all(&wormhole_dir).expect("Fail to create wormhole file in home directory");
                std::fs::write(wormhole_dir.join(SECRET_KEY_FILE), key).expect("Failed to save authentication key file.");

                eprintln!("Authentication key stored successfully!");
                std::process::exit(0);
            },
            None => {
                let key = opts.key;
                let sub_domain = opts.sub_domain;
                let port = opts.port;

                (match key {
                    Some(key) => Some(key),
                    None => {
                        dirs::home_dir()
                            .map(|h| h.join(SETTINGS_DIR).join(SECRET_KEY_FILE))
                            .map(|path| {
                                if path.exists() {
                                    std::fs::read_to_string(path)
                                        .map_err(|e| error!("Error reading authentication token: {:?}", e))
                                        .ok()
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(None)
                    }
                }, sub_domain, port)
            }
        };

        // get the host url
        let tls_off = env::var(TLS_OFF_ENV).is_ok();
        let host = env::var(HOST_ENV)
            .unwrap_or(format!("{}", DEFAULT_HOST));

        let control_host = env::var(HOST_ENV)
            .unwrap_or(format!("{}", DEFAULT_CONTROL_HOST));

        let port = env::var(PORT_ENV)
            .unwrap_or(format!("{}", DEFAULT_CONTROL_PORT));

        let scheme = if tls_off { "ws" } else { "wss" };
        let control_url = format!("{}://{}:{}/wormhole", scheme, control_host, port);

        info!("Control Server URL: {}", &control_url);

        Ok(Config {
            client_id: ClientId::generate(),
            control_url,
            host,
            local_port,
            sub_domain,
            secret_key: secret_key.map(|s| SecretKey(s)),
            tls_off,
            first_run: true,
        })
    }

    pub fn activation_url(&self, server_chosen_sub_domain: &str) -> String {
        format!("{}://{}.{}",
                  if self.tls_off { "http" } else { "https" },
                  &server_chosen_sub_domain,
                  &self.host)
    }
}
