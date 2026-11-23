//! Transport layer: find a device, open it and exchange bytes.
//!
//! Everything behind [`DeviceTransport`] is independent of the kind of
//! connection (USB or Bluetooth), so the bridge and command logic does not need
//! to know about it. The concrete backends (USB vendor/CDC, BLE GATT) are
//! filled in once the transport analysis has confirmed the VID/PIDs, endpoints
//! and UUIDs per machine. Until then this is only the shape.

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("no device found")]
    NotFound,
    #[error("connection lost")]
    Disconnected,
    #[error("timed out after {0:?}")]
    Timeout(Duration),
    #[error("i/o: {0}")]
    Io(String),
    #[error("not implemented yet: {0}")]
    Unimplemented(&'static str),
}

/// How a machine is physically attached. Determines which backend opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// USB as a virtual serial port (CDC-ACM).
    UsbSerial,
    /// USB with a vendor/WinUSB interface (bulk endpoints).
    UsbVendor,
    /// Bluetooth classic, SPP/RFCOMM.
    BluetoothSpp,
    /// Bluetooth Low Energy, GATT.
    Ble,
}

/// What is needed to recognise and open one machine. The concrete values
/// (VID/PID, endpoints, service/characteristic UUIDs) are filled in by the
/// transport analysis; this only states which fields those are.
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub link: Link,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub serial: Option<String>,
    pub label: String,
}

/// An open, raw byte connection to one machine. On top of this
/// `cricut-bridge` builds the session and command logic.
pub trait DeviceTransport: Send {
    /// Write a raw frame towards the machine.
    fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError>;
    /// Read raw bytes from the machine (up to `buf.len()`), with a timeout.
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
}

/// Scans for attached machines. One implementation per backend.
pub trait DeviceScanner {
    fn scan(&self) -> Result<Vec<DeviceDescriptor>, TransportError>;
    fn open(&self, dev: &DeviceDescriptor) -> Result<Box<dyn DeviceTransport>, TransportError>;
}
