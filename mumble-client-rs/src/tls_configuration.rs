use std::error::Error;
use std::fmt::{Debug, Formatter};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls;
use tokio_rustls::rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use tokio_rustls::rustls::client::danger::HandshakeSignatureValid;

pub fn create_root_certificate_store() -> Result<RootCertStore, Box<dyn Error>> {
    let mut cert_store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().expect("Failed to load platform certificates") {
        cert_store.add(cert)?;
    }

    Ok(cert_store)
}

pub struct NoCertificateVerification {}

impl Debug for NoCertificateVerification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Certificate Verification Disabled")
    }
}

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::RSA_PKCS1_SHA1,
             SignatureScheme::ECDSA_SHA1_Legacy,
             SignatureScheme::RSA_PKCS1_SHA256,
             SignatureScheme::ECDSA_NISTP256_SHA256,
             SignatureScheme::RSA_PKCS1_SHA384,
             SignatureScheme::ECDSA_NISTP384_SHA384,
             SignatureScheme::RSA_PKCS1_SHA512,
             SignatureScheme::ECDSA_NISTP521_SHA512,
             SignatureScheme::RSA_PSS_SHA256,
             SignatureScheme::RSA_PSS_SHA384,
             SignatureScheme::RSA_PSS_SHA512,
             SignatureScheme::ED25519,
             SignatureScheme::ED448
        ]
    }
}
