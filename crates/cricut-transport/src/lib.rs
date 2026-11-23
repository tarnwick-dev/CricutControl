//! Transportlaag: een apparaat vinden, openen en bytes uitwisselen.
//!
//! Alles achter [`DeviceTransport`] staat los van het soort verbinding (USB of
//! Bluetooth), zodat de bridge- en commandologica er niets van merkt. De
//! concrete backends (USB-vendor/CDC, BLE GATT) worden ingevuld zodra de
//! transportanalyse de VID/PID's, endpoints en UUID's per machine heeft
//! bevestigd. Tot die tijd is dit alleen de vorm.

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("geen apparaat gevonden")]
    NotFound,
    #[error("verbinding verbroken")]
    Disconnected,
    #[error("time-out na {0:?}")]
    Timeout(Duration),
    #[error("i/o: {0}")]
    Io(String),
    #[error("nog niet ge\u{00ef}mplementeerd: {0}")]
    Unimplemented(&'static str),
}

/// Hoe een machine fysiek hangt. Bepaalt welke backend hem opent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// USB als virtuele seriële poort (CDC-ACM).
    UsbSerial,
    /// USB met een vendor/WinUSB-interface (bulk endpoints).
    UsbVendor,
    /// Bluetooth classic, SPP/RFCOMM.
    BluetoothSpp,
    /// Bluetooth Low Energy, GATT.
    Ble,
}

/// Wat er nodig is om één machine te herkennen en te openen. De concrete
/// waarden (VID/PID, endpoints, service/characteristic-UUID's) vult de
/// transportanalyse in; hier staat alleen welke velden dat zijn.
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub link: Link,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub serial: Option<String>,
    pub label: String,
}

/// Een geopende, ruwe byte-verbinding met één machine. Hierop zet
/// `cricut-bridge` de sessie- en commandologica.
pub trait DeviceTransport: Send {
    /// Schrijf een ruw frame richting de machine.
    fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError>;
    /// Lees ruwe bytes van de machine (tot `buf.len()`), met time-out.
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
}

/// Zoekt naar aangesloten machines. Per backend één implementatie.
pub trait DeviceScanner {
    fn scan(&self) -> Result<Vec<DeviceDescriptor>, TransportError>;
    fn open(&self, dev: &DeviceDescriptor) -> Result<Box<dyn DeviceTransport>, TransportError>;
}
