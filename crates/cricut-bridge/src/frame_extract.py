#!/usr/bin/env python3
"""Print SPP bodies from joyxtra2.pcapng. No keys. direction, enc, hex."""
import struct
import sys

def events_from_pcapng(path):
    f = open(path, "rb").read()
    off = 0
    n = len(f)
    evs = []
    while off + 8 <= n:
        bt = struct.unpack_from("<I", f, off)[0]
        bl = struct.unpack_from("<I", f, off + 4)[0]
        if bl < 12 or off + bl > n:
            break
        if bt == 6:
            body = f[off + 8 : off + bl - 4]
            caplen = struct.unpack_from("<I", body, 12)[0]
            evs.append(body[20 : 20 + caplen])
        off += bl
    return evs

def extract_h4(ev):
    for pos in range(len(ev) - 5):
        L = struct.unpack_from("<I", ev, pos)[0]
        if (
            1 <= L <= 2048
            and pos + 4 + L <= len(ev)
            and ev[pos + 4] in (1, 2, 3, 4)
            and all(b == 0 for b in ev[pos + 4 + L :])
        ):
            direction = ev[pos - 21] if pos >= 21 else 0
            return ev[pos + 4 : pos + 4 + L], (1 if direction == 1 else 0)
    return None, None

def main():
    evs = events_from_pcapng(sys.argv[1])
    records = []
    for ev in evs:
        h4, direction = extract_h4(ev)
        if h4 is not None:
            records.append((h4, direction))
    streams = {}
    frames = []

    def feed(h4):
        if not h4 or h4[0] != 2 or len(h4) < 5:
            return None
        body = h4[1:]
        hw = struct.unpack_from("<H", body, 0)[0]
        dlen = struct.unpack_from("<H", body, 2)[0]
        payload = body[4 : 4 + dlen]
        if len(payload) != dlen:
            return None
        handle = hw & 0x0FFF
        pb = (hw >> 12) & 0x3
        if pb == 1:
            streams.setdefault(handle, bytearray()).extend(payload)
        else:
            streams[handle] = bytearray(payload)
        buf = streams[handle]
        if len(buf) < 4:
            return None
        llen, _cid = struct.unpack_from("<HH", buf, 0)
        if len(buf) < 4 + llen:
            return None
        sdu = bytes(buf[4 : 4 + llen])
        streams[handle] = bytearray(buf[4 + llen :])
        return sdu

    for h4, direction in records:
        sdu = feed(h4)
        if not sdu or len(sdu) < 4:
            continue
        ctrl = sdu[1]
        if ctrl not in (0xEF, 0xFF):
            continue
        if sdu[2] & 1:
            length = sdu[2] >> 1
            offi = 3
        else:
            length = (sdu[2] >> 1) | (sdu[3] << 7)
            offi = 4
        if ctrl == 0xFF:
            offi += 1
        if offi + length > len(sdu):
            continue
        info = sdu[offi : offi + length]
        if ((sdu[0] >> 2) & 0x3F) == 0 or not info:
            continue
        b = info
        i = 0
        while i + 2 <= len(b):
            hdr = (b[i] << 8) | b[i + 1]
            ln = hdr & 0x7FFF
            if ln == 0 or ln > 0x400 or i + 2 + ln > len(b):
                break
            frames.append((direction, bool(hdr & 0x8000), b[i + 2 : i + 2 + ln]))
            i += 2 + ln
    enc = sum(1 for f in frames if f[1])
    print(f"hci {len(records)} spp {len(frames)} enc {enc}", file=sys.stderr)
    for direction, encrypted, body in frames:
        print(f"{direction}\t{1 if encrypted else 0}\t{body.hex()}")

if __name__ == "__main__":
    main()
