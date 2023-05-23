use rustls::{Error as RustTlsError, client::{ServerCertVerifier, ServerCertVerified, HandshakeSignatureValid}};

pub struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        server_name: &rustls::ServerName,
        scts: &mut dyn Iterator<Item = &[u8]>,
        ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> std::result::Result<rustls::client::ServerCertVerified, RustTlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &rustls::Certificate,
            dss: &rustls::DigitallySignedStruct,
        ) -> std::result::Result<rustls::client::HandshakeSignatureValid, RustTlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &rustls::Certificate,
            dss: &rustls::DigitallySignedStruct,
        ) -> std::result::Result<rustls::client::HandshakeSignatureValid, RustTlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }
}