use std::error::Error;
use tokio_rustls::rustls;
use tokio_rustls::rustls::RootCertStore;

pub fn create_root_certificate_store() -> Result<RootCertStore, Box<dyn Error>> {
    let mut cert_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("Failed to load platform certificates") {
        cert_store.add(&rustls::Certificate(cert.0))?;
    }

    Ok(cert_store)
}

pub struct NoCertificateVerification {}

impl rustls::client::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
