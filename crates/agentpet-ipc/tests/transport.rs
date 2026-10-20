//! Transport-level tests for the local daemon IPC.
//!
//! The tests bind a private endpoint, so they never touch the real
//! daemon endpoint and never require a running daemon.

use std::sync::atomic::{AtomicU64, Ordering};

use agentpet_ipc::{client, connect, DaemonServer, Endpoint, IpcErrorCode, LocalSecret};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Builds a private endpoint that no other test shares.
fn test_endpoint() -> Endpoint {
    let index = COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!("{}-{}", std::process::id(), index);

    #[cfg(windows)]
    {
        Endpoint::NamedPipe(format!(r"\\.\pipe\agentpet-ipc-test-{unique}"))
    }

    #[cfg(unix)]
    {
        let path = std::env::temp_dir().join(format!("agentpet-ipc-test-{unique}.sock"));
        Endpoint::UnixSocket(path)
    }
}

fn secret() -> LocalSecret {
    LocalSecret::generate().expect("secret generated")
}

#[tokio::test]
async fn authenticates_a_matching_peer() {
    let endpoint = test_endpoint();
    let shared = secret();
    let server_secret = shared.clone();
    let mut server = DaemonServer::bind(&endpoint).await.expect("server bound");

    let server_task = tokio::spawn(async move {
        let mut connection = server.accept().await.expect("connection accepted");
        connection
            .authenticate_server(server_secret.as_bytes())
            .await
            .expect("server handshake");
        connection.write_frame(b"pong").await.expect("reply sent");
    });

    let mut client = connect(&endpoint, shared.as_bytes())
        .await
        .expect("client connected");
    client.write_frame(b"ping").await.expect("request sent");
    let reply = client
        .read_frame()
        .await
        .expect("reply read")
        .expect("reply present");
    assert_eq!(reply, b"pong");

    server_task.await.expect("server task finished");
}

#[tokio::test]
async fn rejects_a_peer_with_the_wrong_secret() {
    let endpoint = test_endpoint();
    let server_secret = secret();
    let client_secret = secret();
    let mut server = DaemonServer::bind(&endpoint).await.expect("server bound");

    let server_task = tokio::spawn(async move {
        let mut connection = server.accept().await.expect("connection accepted");
        let result = connection
            .authenticate_server(server_secret.as_bytes())
            .await;
        assert_eq!(
            result.expect_err("handshake rejected").code(),
            IpcErrorCode::AuthenticationFailed
        );
    });

    let error = connect(&endpoint, client_secret.as_bytes())
        .await
        .expect_err("client rejected");
    assert_eq!(error.code(), IpcErrorCode::AuthenticationFailed);

    server_task.await.expect("server task finished");
}

#[cfg(unix)]
#[tokio::test]
async fn restricts_the_unix_socket_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;

    let endpoint = test_endpoint();
    let server = DaemonServer::bind(&endpoint).await.expect("server bound");
    let path = endpoint.socket_path().expect("socket path").to_path_buf();
    let mode = std::fs::metadata(&path)
        .expect("socket metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "socket must not be reachable by other users");
    drop(server);
}

#[cfg(unix)]
#[tokio::test]
async fn removes_the_socket_file_on_shutdown() {
    let endpoint = test_endpoint();
    let path = endpoint.socket_path().expect("socket path").to_path_buf();
    {
        let _server = DaemonServer::bind(&endpoint).await.expect("server bound");
        assert!(path.exists(), "socket file exists while the server runs");
    }
    assert!(!path.exists(), "socket file is removed on shutdown");
}

#[tokio::test]
async fn reports_an_unavailable_endpoint() {
    let endpoint = test_endpoint();
    let shared = secret();
    let error = connect(&endpoint, shared.as_bytes())
        .await
        .expect_err("no daemon is listening");
    assert_eq!(error.code(), IpcErrorCode::TransportUnavailable);
}

#[tokio::test]
async fn carries_json_documents_over_frames() {
    let endpoint = test_endpoint();
    let shared = secret();
    let server_secret = shared.clone();
    let mut server = DaemonServer::bind(&endpoint).await.expect("server bound");

    let server_task = tokio::spawn(async move {
        let mut connection = server.accept().await.expect("connection accepted");
        connection
            .authenticate_server(server_secret.as_bytes())
            .await
            .expect("server handshake");
        let request: serde_json::Value = connection.read_json().await.expect("request decoded");
        let reply = serde_json::json!({ "echo": request["value"] });
        connection.write_json(&reply).await.expect("reply sent");
    });

    let mut client = client::connect_unauthenticated(&endpoint)
        .await
        .expect("client connected");
    client
        .authenticate_client(shared.as_bytes())
        .await
        .expect("client handshake");
    client
        .write_json(&serde_json::json!({ "value": 7 }))
        .await
        .expect("request sent");
    let reply: serde_json::Value = client.read_json().await.expect("reply decoded");
    assert_eq!(reply["echo"], serde_json::json!(7));

    server_task.await.expect("server task finished");
}
