//! SHA-256 (Magvault's integrity hash) and SHA-512 / C4 ID (the hash format ASC MHL
//! requires for its chain file, and the one Magvault writes into its MHL manifests,
//! because ASC MHL 1.0 does not define SHA-256).
//!
//! Plain FIPS 180-4 implementations; constants live in `hash_consts.rs`.

use crate::hash_consts::{H256, H512, K256, K512};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    len: usize,
    total: u64,
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 { h: H256, buf: [0; 64], len: 0, total: 0 }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if self.len > 0 {
            let take = (64 - self.len).min(data.len());
            self.buf[self.len..self.len + take].copy_from_slice(&data[..take]);
            self.len += take;
            data = &data[take..];
            if self.len == 64 {
                let block = self.buf;
                compress256(&mut self.h, &block);
                self.len = 0;
            }
        }
        while data.len() >= 64 {
            compress256(&mut self.h, data[..64].try_into().unwrap());
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let bits = self.total.wrapping_mul(8);
        self.update_pad(&[0x80]);
        while self.len != 56 {
            self.update_pad(&[0]);
        }
        self.update_pad(&bits.to_be_bytes());
        let mut out = [0u8; 32];
        for (i, w) in self.h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    // Like update, but does not count towards the message length.
    fn update_pad(&mut self, data: &[u8]) {
        let t = self.total;
        self.update(data);
        self.total = t;
    }
}

fn compress256(h: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *x = x.wrapping_add(y);
    }
}

pub struct Sha512 {
    h: [u64; 8],
    buf: [u8; 128],
    len: usize,
    total: u128,
}

impl Sha512 {
    pub fn new() -> Self {
        Sha512 { h: H512, buf: [0; 128], len: 0, total: 0 }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u128);
        if self.len > 0 {
            let take = (128 - self.len).min(data.len());
            self.buf[self.len..self.len + take].copy_from_slice(&data[..take]);
            self.len += take;
            data = &data[take..];
            if self.len == 128 {
                let block = self.buf;
                compress512(&mut self.h, &block);
                self.len = 0;
            }
        }
        while data.len() >= 128 {
            compress512(&mut self.h, data[..128].try_into().unwrap());
            data = &data[128..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 64] {
        let bits = self.total.wrapping_mul(8);
        self.update_pad(&[0x80]);
        while self.len != 112 {
            self.update_pad(&[0]);
        }
        self.update_pad(&bits.to_be_bytes());
        let mut out = [0u8; 64];
        for (i, w) in self.h.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    fn update_pad(&mut self, data: &[u8]) {
        let t = self.total;
        self.update(data);
        self.total = t;
    }
}

fn compress512(h: &mut [u64; 8], block: &[u8; 128]) {
    let mut w = [0u64; 80];
    for i in 0..16 {
        w[i] = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().unwrap());
    }
    for i in 16..80 {
        let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
        let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..80 {
        let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K512[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *x = x.wrapping_add(y);
    }
}

pub fn hex(bytes: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(D[(b >> 4) as usize] as char);
        s.push(D[(b & 15) as usize] as char);
    }
    s
}

const B58: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// C4 ID (SMPTE ST 2114): "c4" + Base58 of the SHA-512 digest read as a big-endian
/// integer, left-padded with '1' (Base58 zero) to 88 characters. 90 characters total.
pub fn c4_id(sha512: &[u8; 64]) -> String {
    let mut num = sha512.to_vec();
    let mut digits: Vec<u8> = Vec::with_capacity(88);
    while num.iter().any(|&b| b != 0) {
        let mut rem: u32 = 0;
        for byte in num.iter_mut() {
            let cur = (rem << 8) | *byte as u32;
            *byte = (cur / 58) as u8;
            rem = cur % 58;
        }
        digits.push(B58[rem as usize]);
    }
    while digits.len() < 88 {
        digits.push(b'1');
    }
    digits.reverse();
    let mut s = String::from("c4");
    s.push_str(std::str::from_utf8(&digits).unwrap());
    s
}

#[cfg(test)]
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex(&h.finalize())
}

pub fn c4_of_bytes(data: &[u8]) -> String {
    let mut h = Sha512::new();
    h.update(data);
    c4_id(&h.finalize())
}

/// Hashes of one file from a single read.
pub struct FileHashes {
    pub sha256: String,
    pub c4: Option<String>,
    pub bytes: u64,
}

const BUF: usize = 4 * 1024 * 1024;

/// Read a file once, computing SHA-256 and (optionally) SHA-512 for C4.
/// `progress` is called with the number of bytes read since the last call.
pub fn hash_file(path: &Path, with_c4: bool, progress: &mut dyn FnMut(u64)) -> std::io::Result<FileHashes> {
    let mut f = File::open(path)?;
    let mut buf = vec![0u8; BUF];
    let mut s256 = Sha256::new();
    let mut s512 = if with_c4 { Some(Sha512::new()) } else { None };
    let mut bytes = 0u64;
    loop {
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        s256.update(&buf[..n]);
        if let Some(s) = s512.as_mut() {
            s.update(&buf[..n]);
        }
        bytes += n as u64;
        progress(n as u64);
    }
    Ok(FileHashes {
        sha256: hex(&s256.finalize()),
        c4: s512.map(|s| c4_id(&s.finalize())),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha512_hex(data: &[u8]) -> String {
        let mut h = Sha512::new();
        h.update(data);
        hex(&h.finalize())
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_streaming_matches_one_shot() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 31 % 251) as u8).collect();
        let one = sha256_hex(&data);
        for chunk in [1usize, 3, 63, 64, 65, 127, 1000] {
            let mut h = Sha256::new();
            for c in data.chunks(chunk) {
                h.update(c);
            }
            assert_eq!(hex(&h.finalize()), one, "chunk {chunk}");
        }
    }

    #[test]
    fn sha512_known_vectors() {
        assert_eq!(
            sha512_hex(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            sha512_hex(b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
    }

    /// Multi-buffer file (> 4 MiB read buffer) checked against coreutils, when present.
    #[test]
    fn large_file_matches_coreutils() {
        let path = std::env::temp_dir().join(format!("mv-hash-{}", std::process::id()));
        let data: Vec<u8> = (0..9_437_187u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        let mut seen = 0;
        let h = hash_file(&path, true, &mut |n| seen += n).unwrap();
        assert_eq!(seen, data.len() as u64);
        let tool = |name: &str| {
            std::process::Command::new(name).arg(&path).output().ok().filter(|o| o.status.success()).map(|o| {
                String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap_or("").to_string()
            })
        };
        if let Some(s) = tool("sha256sum") {
            assert_eq!(h.sha256, s);
        }
        if let Some(s) = tool("sha512sum") {
            assert_eq!(sha512_hex(&data), s);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn c4_shape() {
        let id = c4_of_bytes(b"");
        assert_eq!(id.len(), 90);
        assert!(id.starts_with("c4"));
        assert!(id[2..].bytes().all(|b| B58.contains(&b)));
    }
}
