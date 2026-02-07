use std::net::SocketAddr;
use x_one::Server;
use x_one::xserver::axum::*;

    #[test]
    fn test_axum_server_with_addr() {
        let router = axum::Router::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AxumServer::with_addr(router, addr);
        assert_eq!(server.addr(), addr);
    }

    #[tokio::test]
    async fn test_axum_tls_server_not_implemented() {
        let router = axum::Router::new();
        let server = AxumTlsServer::new(router, "cert.pem", "key.pem");
        let result = server.run().await;
        assert!(result.is_err());
    }
