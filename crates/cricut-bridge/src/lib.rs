//! Machinesessie: de laag tussen een ruwe [`DeviceTransport`] en de commando's.
//!
//! Wij praten rechtstreeks met de machine en nemen dus de rol van de Design
//! Space-helper over. Twee dingen zitten bewust achter een trait, omdat ze pas
//! ingevuld worden als we ze van onze eigen machine hebben bevestigd:
//!
//! * [`FrameCodec`] — de framing richting de machine (lengte-header, byte-orde,
//!   checksum). De exacte vorm komt uit de transportanalyse.
//! * [`SessionCipher`] — de versleuteling van de verbinding. Deze repo bevat
//!   geen sleutels en geen uit Design Space overgenomen cryptocode. De
//!   implementatie en het sleutelmateriaal komen los, uit een opname van onze
//!   eigen machine. Zie `docs/protocol.md`, hoofdstuk Versleuteling.

use cricut_transport::{DeviceTransport, TransportError};

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("protocolfout: {0}")]
    Protocol(String),
    #[error("nog niet ge\u{00ef}mplementeerd: {0}")]
    Unimplemented(&'static str),
}

/// Omhullen en uit elkaar halen van frames op de lijn naar de machine.
/// Pending: de concrete header/lengte/checksum uit de transportanalyse.
pub trait FrameCodec {
    /// Zet een payload in een frame klaar om te verzenden.
    fn encode(&self, payload: &[u8], out: &mut Vec<u8>) -> Result<(), BridgeError>;
    /// Haal één compleet frame uit de inkomende buffer, indien aanwezig.
    fn decode(&self, inbuf: &mut Vec<u8>) -> Result<Option<Vec<u8>>, BridgeError>;
}

/// De versleutelde laag op de verbinding. Implementatie en sleutels komen los
/// (zie modulebeschrijving). De no-op-variant is voor de plaintext-fase van de
/// handshake en voor testen op een opname.
pub trait SessionCipher {
    fn encrypt(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError>;
    fn decrypt(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError>;
}

/// Geen versleuteling: voor de open setup-fase en voor replay-tests.
pub struct NullCipher;
impl SessionCipher for NullCipher {
    fn encrypt(&mut self, plain: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(plain.to_vec())
    }
    fn decrypt(&mut self, cipher: &[u8]) -> Result<Vec<u8>, BridgeError> {
        Ok(cipher.to_vec())
    }
}

/// Een lopende sessie met één machine.
pub struct MachineSession<T: DeviceTransport, C: FrameCodec, S: SessionCipher> {
    transport: T,
    codec: C,
    cipher: S,
}

impl<T: DeviceTransport, C: FrameCodec, S: SessionCipher> MachineSession<T, C, S> {
    pub fn new(transport: T, codec: C, cipher: S) -> Self {
        Self { transport, codec, cipher }
    }

    /// Plaatshouder voor de opstart: statuslezen, registers, sleuteluitwisseling.
    /// Wordt ingevuld na de transportanalyse en de crypto-opname.
    pub fn handshake(&mut self) -> Result<(), BridgeError> {
        let _ = (&mut self.transport, &mut self.codec, &mut self.cipher);
        Err(BridgeError::Unimplemented("handshake"))
    }
}
