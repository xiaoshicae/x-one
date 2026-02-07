use std::net::SocketAddr;
use x_one::Server;
use x_one::xserver::auxm::*;

    #[test]
    fn test_auxm_server_with_addr() {
        let router = axum::Router::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = AuxmServer::with_addr(router, addr);
        assert_eq!(server.addr(), addr);
    }

    #[tokio::test]
    async fn test_auxm_tls_server_not_implemented() {
        let router = axum::Router::new();
        let server = AuxmTlsServer::new(router, "cert.pem", "key.pem");
        let result = server.run().await;
        assert!(result.is_err());
    }
