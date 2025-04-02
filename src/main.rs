use anyhow::bail;
use clap::Parser;
use iroh::{
    Endpoint, NodeId, SecretKey,
    endpoint::{ApplicationClose, ConnectionError, RecvStream, SendStream, VarInt},
};
use std::{io::ErrorKind, path::PathBuf, process::exit};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
};

mod args;

const ALPN: &[u8] = b"do-ssh";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = args::Cli::parse();

    match cli.sub {
        args::Sub::Client { node_id, port } => {
            //  We take special care because ssh does not like Rust's error handling
            if let Err(err) = client(node_id, port).await {
                eprintln!("{}", err);
                exit(1);
            } else {
                exit(0);
            }
        }
        args::Sub::Server { key_file, create } => {
            if let Err(err) = server(key_file, create).await {
                eprintln!("{}", err);
                eprintln!("Server crashed!");
                exit(1);
            }
            unreachable!();
        }
        args::Sub::Gen {
            key_file,
            r#override,
        } => generate(key_file, r#override).await,
    }
}

async fn generate(key_file: PathBuf, r#override: bool) -> anyhow::Result<()> {
    if key_file.is_file() && !r#override {
        bail!("The keyfile already exists and will not be overwritten.");
    }
    println!(
        "Generating new key at {}",
        key_file
            .file_name()
            .and_then(|val| val.to_str())
            .unwrap_or("None")
    );
    let bytes: [u8; 32] = rand::random();
    tokio::fs::write(&key_file, &bytes).await?;
    Ok(())
}

async fn client(node_id: NodeId, port: u16) -> anyhow::Result<()> {
    let ep = Endpoint::builder()
        //.secret_key(key)
        .alpns(vec![ALPN.to_vec()])
        .discovery_n0()
        .discovery_local_network()
        .bind()
        .await?;
    let connection = ep.connect(node_id, ALPN).await?;
    let (mut tx, mut rx) = connection.open_bi().await?;
    tx.write_u16(port).await?;
    enum Handle {
        Stdout(String),
        Stdin(String),
        Closed(ConnectionError),
    }
    let (ctx, mut crx) = tokio::sync::mpsc::channel(1);
    let ctx_ = ctx.clone();
    let h1 = tokio::spawn(async move {
        if let Err(err) = tokio::io::copy(&mut rx, &mut tokio::io::stdout()).await {
            _ = ctx_.send(Handle::Stdout(err.to_string())).await;
        }
    });
    let ctx_ = ctx.clone();
    let h2 = tokio::spawn(async move {
        if let Err(err) = tokio::io::copy(&mut tokio::io::stdin(), &mut tx).await {
            _ = ctx_.send(Handle::Stdin(err.to_string())).await;
        }
    });
    let conn = connection.clone();
    let h3 = tokio::spawn(async move {
        let closed = conn.closed().await;
        _ = ctx.send(Handle::Closed(closed)).await;
    });
    let err = crx.recv().await.unwrap();
    let res = match err {
        Handle::Stdout(err) | Handle::Stdin(err) => {
            connection.close(VarInt::from(0u8), b"");
            eprintln!("Connection aborted");
            Err(anyhow::Error::msg(err))
        }
        Handle::Closed(closed) => match closed {
            ConnectionError::ApplicationClosed(ApplicationClose { error_code, reason })
                if error_code.clone().into_inner() == 1 =>
            {
                Err(anyhow::Error::msg(
                    String::from_utf8_lossy(&reason).to_string(),
                ))
            }
            closed => Err(anyhow::Error::msg(closed.to_string())),
        },
    };
    h1.abort();
    h2.abort();
    h3.abort();
    res
}

async fn server(key_file: PathBuf, create: bool) -> anyhow::Result<()> {
    let key = match tokio::fs::read(&key_file).await {
        Ok(file) => {
            //  Parse key
            let bytes: [u8; 32] = (*file.into_boxed_slice()).try_into()?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(err) if err.kind() == ErrorKind::NotFound && create => {
            //  Generate new key
            println!("Generating new key at {}", key_file.to_str().unwrap_or(""));
            let bytes: [u8; 32] = rand::random();
            tokio::fs::write(&key_file, &bytes).await?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        //  Fuck
        Err(err) => Err(anyhow::Error::new(err)),
    }?;
    let ep = Endpoint::builder()
        .secret_key(key)
        .alpns(vec![ALPN.to_vec()])
        .discovery_n0()
        .discovery_local_network();
    let ep = ep.bind().await?;
    println!("NodeId: {}", ep.node_id());
    loop {
        //  Can be unwrapped, because we won't be here if the endpoint gets closed
        //    (This only returns None if the endpoint is closed)
        let connection = ep.accept().await.unwrap();
        tokio::spawn(async move {
            let connection = match connection.await {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
            let (tx, rx) = match connection.accept_bi().await {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("{}", err);
                    return;
                }
            };
            println!("Got connection: {}", connection.remote_node_id().unwrap());
            if let Err(err) = handle_connection(tx, rx).await {
                connection.close(VarInt::from(1u8), &format!("{}", err).into_bytes());
                _ = connection.closed().await;

                eprintln!("{}", err);
            } else {
                connection.close(VarInt::from(0u8), &[]);
            }
            println!(
                "Closed connection: {}",
                connection.remote_node_id().unwrap()
            );
        });
    }
}

async fn handle_connection(mut tx: SendStream, mut rx: RecvStream) -> anyhow::Result<()> {
    let port = rx.read_u16().await?;
    let mut tcp = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
    println!("Created new connection with port {}", port);
    let (mut tcprx, mut tcptx) = tcp.split();

    let (res1, res2) = join!(
        tokio::io::copy(&mut tcprx, &mut tx),
        tokio::io::copy(&mut rx, &mut tcptx)
    );
    res1?;
    res2?;
    Ok(())
}
