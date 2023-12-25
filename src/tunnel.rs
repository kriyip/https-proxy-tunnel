use crate::utils::TunnelType;
use std::net::SocketAddr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct Tunnel {
    listener: TcpListener,
}

impl Tunnel {
    pub async fn new(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(Self { listener: listener })
    }

    pub async fn run(&self) -> io::Result<()> {
        loop {
            let (socket, _) = self.listener.accept().await?;
            tokio::spawn(async move {
                if let Err(e) = handle_tcp(socket).await {
                    eprintln!("failed to process connection; error = {}", e);
                }
            });
        }
    }
}

async fn handle_tcp(mut client_socket: TcpStream) -> io::Result<()> {
    println!("new connection from {}", client_socket.peer_addr()?);

    // 1024 byte buffer to read from tcp stream
    let mut buffer = [0; 1024];
    let n = client_socket.read(&mut buffer).await?;

    if (n == 0) {
        return Ok(());
    }

    println!("received {} bytes", n);

    // get destination from tcp stream
    let destination = std::str::from_utf8(&buffer[..n])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid address format"))?;

    println!("destination str: {}", destination);
    let dest_server_addr: SocketAddr = destination
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid address"))?;

    println!("destination socket: {}", dest_server_addr);

    let mut dest_server_socket = TcpStream::connect(dest_server_addr).await?;

    // create a stream to client and to destination
    let (mut client_reader, mut client_writer) = client_socket.split();
    let (mut server_reader, mut server_writer) = dest_server_socket.split();

    let client_to_server_stream = io::copy(&mut client_reader, &mut server_writer);
    let server_to_client_stream = io::copy(&mut server_reader, &mut client_writer);

    tokio::select! {
        result = client_to_server_stream => {
            result?;
        },
        result = server_to_client_stream => {
            result?;
        }
    }
    Ok(())
}

// handles an HTTP Connect request
async fn handle_connect(mut client_socket: TcpStream) -> io::Result<()> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // start an echo server on a localhost port
    async fn start_mock_server() -> io::Result<SocketAddr> {
        let listener: TcpListener = TcpListener::bind("127.0.0.1:0").await?;
        let server_addr = listener.local_addr()?;

        // server logic (echoes back whatever it receives)
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    if n > 0 {
                        println!("message received: {}", String::from_utf8_lossy(&buf[..n]));
                        let _ = socket.write_all(&buf[..n]).await;
                    }
                }
            }
        });
        println!("mock server running on {}", server_addr);

        Ok(server_addr)
    }

    #[tokio::test]
    async fn test_tcp_tunnel() -> io::Result<()> {
        // Start a mock server
        let server_addr = start_mock_server().await?;

        // Start the tunnel
        let tunnel_addr_str = "127.0.0.1:4444";
        let tunnel = Tunnel::new(tunnel_addr_str).await?;
        tokio::spawn(async move {
            let _ = tunnel.run().await;
        });

        println!("tunnel running on {}", tunnel_addr_str);

        // Connect a client to the tunnel and send data to the mock server
        let mut client = TcpStream::connect(tunnel_addr_str).await?;
        client
            .write_all(format!("{}:{}\n", server_addr.ip(), server_addr.port()).as_bytes())
            .await?;
        client.write_all(b"Hello, server!").await?;

        // Read the response (which should be an echo of the sent data)
        let mut response = vec![0; 1024];
        let n = client.read(&mut response).await?;
        assert!(n > 0);
        assert_eq!(&response[..n], b"Hello, server!");

        Ok(())
    }
}
