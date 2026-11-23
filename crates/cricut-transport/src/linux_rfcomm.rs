//! Classic Bluetooth RFCOMM transport on Linux (BlueZ), via raw sockets.
//!
//! The Joy Xtra is classic Bluetooth SPP on RFCOMM channel 4. This opens an
//! RFCOMM socket to a paired device and gives a byte stream to the bridge layer.
//! Pair the device first (bluetoothctl); this does not pair.

use std::io;
use std::os::fd::RawFd;
use std::time::Duration;

use crate::{DeviceTransport, TransportError};

const AF_BLUETOOTH: libc::c_int = 31;
const BTPROTO_RFCOMM: libc::c_int = 3;

#[repr(C)]
struct SockaddrRc {
    rc_family: libc::sa_family_t,
    rc_bdaddr: [u8; 6], // little-endian
    rc_channel: u8,
}

/// Parse "FC:97:A8:66:6C:6C" into the little-endian 6-byte form BlueZ wants.
fn parse_bdaddr(s: &str) -> Result<[u8; 6], TransportError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(TransportError::Io(format!("bad BD_ADDR: {s}")));
    }
    let mut be = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        be[i] = u8::from_str_radix(p, 16).map_err(|_| TransportError::Io(format!("bad BD_ADDR: {s}")))?;
    }
    be.reverse(); // BlueZ stores it little-endian
    Ok(be)
}

/// An open RFCOMM connection to a machine.
pub struct RfcommTransport {
    fd: RawFd,
}

impl RfcommTransport {
    /// Connect to `bdaddr` on `channel` (4 for the Joy Xtra). Must already be paired.
    pub fn connect(bdaddr: &str, channel: u8) -> Result<Self, TransportError> {
        let addr = parse_bdaddr(bdaddr)?;
        let fd = unsafe { libc::socket(AF_BLUETOOTH, libc::SOCK_STREAM, BTPROTO_RFCOMM) };
        if fd < 0 {
            return Err(TransportError::Io(format!("socket: {}", io::Error::last_os_error())));
        }
        let sa = SockaddrRc { rc_family: AF_BLUETOOTH as libc::sa_family_t, rc_bdaddr: addr, rc_channel: channel };
        let rc = unsafe {
            libc::connect(
                fd,
                &sa as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrRc>() as libc::socklen_t,
            )
        };
        if rc < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(match e.raw_os_error() {
                Some(112) | Some(111) => TransportError::NotFound, // host down / refused
                _ => TransportError::Io(format!("connect: {e}")),
            });
        }
        Ok(RfcommTransport { fd })
    }

    fn set_read_timeout(&self, t: Duration) {
        let tv = libc::timeval {
            tv_sec: t.as_secs() as libc::time_t,
            tv_usec: t.subsec_micros() as libc::suseconds_t,
        };
        unsafe {
            libc::setsockopt(
                self.fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &tv as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }
    }
}

impl DeviceTransport for RfcommTransport {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError> {
        let n = unsafe { libc::write(self.fd, bytes.as_ptr() as *const libc::c_void, bytes.len()) };
        if n < 0 {
            return Err(TransportError::Io(format!("write: {}", io::Error::last_os_error())));
        }
        Ok(n as usize)
    }

    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        self.set_read_timeout(timeout);
        let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            return match e.raw_os_error() {
                Some(libc::EAGAIN) => Err(TransportError::Timeout(timeout)),
                _ => Err(TransportError::Io(format!("read: {e}"))),
            };
        }
        Ok(n as usize)
    }
}

impl Drop for RfcommTransport {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}
