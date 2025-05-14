use clap::{ArgAction, Parser, Subcommand};
use iroh::NodeId;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub sub: Sub,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Sub {
    /// Start a server
    Server {
        /// The path of the keyfile to use
        #[arg(
            short = 'k',
            short_aliases = ['i', 'f'],
            long = "key",
            default_value = "key.priv"
        )]
        key_file: PathBuf,
        /// Prevent generating a new keyfile if none is found
        #[arg(short = 'n', long = "no-create", action = ArgAction::SetFalse)]
        create: bool,
        #[arg(short = 'p', long = "port", num_args = 0.., value_delimiter = ',', default_values_t = [22u16])]
        ports: Vec<u16>,
    },
    /// Connect as a client
    Client {
        /// The iroh NodeId to connect to
        #[arg()]
        node_id: NodeId,
        /// The port of the ssh server
        #[arg(default_value_t = 22)]
        port: u16,
    },
    /// Generate a new private key (which implies the node id)
    #[command(alias = "gen")]
    Generate {
        /// Path to the keyfile to generate
        #[arg(
            short = 'k',
            short_aliases = ['i', 'f'],
            long = "key",
            default_value = "key.priv"
        )]
        key_file: PathBuf,
        /// Allow replacing an existing file
        #[arg(short = 'o', long = "override")]
        r#override: bool,
    },
    /// Extract public key (NodeId) from private key
    #[command(alias = "pub")]
    Nodeid {
        /// Path to the private keyfile
        #[arg(
            short = 'k',
            short_aliases = ['i', 'f'],
            long = "key",
            default_value = "key.priv"
        )]
        key_file: PathBuf,
        /// Path to save the public key (optional)
        #[arg(short = 'o', long = "output")]
        output_file: Option<PathBuf>,
    },
}