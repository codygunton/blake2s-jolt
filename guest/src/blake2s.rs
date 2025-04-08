pub const BLOCK_BYTES : usize = 64;
pub const OUT_BYTES   : usize = 32;
pub const KEY_BYTES   : usize = 32;

static IV : [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

static SIGMA : [[u8; 16]; 10] = [
    [  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15 ],
    [ 14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3 ],
    [ 11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4 ],
    [  7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8 ],
    [  9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13 ],
    [  2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9 ],
    [ 12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11 ],
    [ 13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10 ],
    [  6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5 ],
    [ 10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13 , 0 ],
];

pub struct Blake2s {
    h: [u32; 8],
    t: [u32; 2],
    f: [u32; 2],
    buf: [u8; 2*BLOCK_BYTES],
    buf_len: usize,
}

impl Copy for Blake2s {}
impl Clone for Blake2s { fn clone(&self) -> Blake2s { *self } }

impl Blake2s {
    pub fn new(size: usize) -> Blake2s {
        assert!(size > 0 && size <= OUT_BYTES);

        let param = encode_params(size as u8, 0);
        let mut state = IV;

        for i in 0..state.len() {
            state[i] ^= load32(&param[i*4..]);
        }

        Blake2s{
            h: state,
            t: [0, 0],
            f: [0, 0],
            buf: [0u8; 2*BLOCK_BYTES],
            buf_len: 0,
        }
    }

    pub fn new_with_key(size: usize, key: &[u8]) -> Blake2s {
        assert!(size > 0 && size <= OUT_BYTES);
        assert!(key.len() > 0 && key.len() <= KEY_BYTES);

        let param = encode_params(size as u8, key.len() as u8);
        let mut state = IV;

        for i in 0..state.len() {
            state[i] ^= load32(&param[i*4..]);
        }

        let mut b = Blake2s{
            h: state,
            t: [0, 0],
            f: [0, 0],
            buf: [0u8; 2*BLOCK_BYTES],
            buf_len: 0,
        };

        let mut block = [0u8; BLOCK_BYTES];
        for i in 0..key.len() {
            block[i] = key[i];
        }
        b.update(block.as_ref());
        b
    }

    pub fn update(&mut self, m: &[u8]) {
        let mut m = m;

        while m.len() > 0 {
            let left = self.buf_len;
            let fill = 2 * BLOCK_BYTES - left;

            if m.len() > fill {
                for i in 0..fill {
                    self.buf[left+i] = m[i];
                }
                self.buf_len += fill;
                m = &m[fill..];
                self.increment_counter(BLOCK_BYTES as u32);
                self.compress();
                for i in 0..BLOCK_BYTES {
                    self.buf[i] = self.buf[i+BLOCK_BYTES];
                }
                self.buf_len -= BLOCK_BYTES;
            } else {
                for i in 0..m.len() {
                    self.buf[left+i] = m[i];
                }
                self.buf_len += m.len();
                m = &m[m.len()..];
            }
        }
    }

    pub fn finalize(&mut self, out: &mut [u8]) {
        let mut buf = [0u8; OUT_BYTES];
        if self.buf_len > BLOCK_BYTES {
            self.increment_counter(BLOCK_BYTES as u32);
            self.compress();
            for i in 0..BLOCK_BYTES {
                self.buf[i] = self.buf[i+BLOCK_BYTES];
            }
            self.buf_len -= BLOCK_BYTES;
        }
        let n = self.buf_len as u32;
        self.increment_counter(n);
        self.f[0] = !0;
        for i in self.buf_len..self.buf.len() {
            self.buf[i] = 0;
        }
        self.compress();
        for i in 0..self.h.len() {
            store32(&mut buf[i*4..], self.h[i]);
        }

        for i in 0..::std::cmp::min(out.len(), OUT_BYTES) {
            out[i] = buf[i];
        }
    }

    fn increment_counter(&mut self, inc: u32) {
        self.t[0] += inc;
        self.t[1] += if self.t[0] < inc {1} else {0};
    }

    fn compress(&mut self) {
        let mut m = [0u32; 16];
        let mut v = [0u32; 16];
        let block = self.buf.as_ref();

        assert!(block.len() >= BLOCK_BYTES);

        for i in 0..m.len() {
            m[i] = load32(&block[i*4..]);
        }

        for i in 0..8 {
            v[i] = self.h[i];
        }

        v[ 8] = IV[0];
        v[ 9] = IV[1];
        v[10] = IV[2];
        v[11] = IV[3];
        v[12] = self.t[0] ^ IV[4];
        v[13] = self.t[1] ^ IV[5];
        v[14] = self.f[0] ^ IV[6];
        v[15] = self.f[1] ^ IV[7];

        macro_rules! g(
            ($r: expr, $i: expr, $a: expr, $b: expr, $c: expr, $d: expr) => ({
                $a = $a.wrapping_add($b).wrapping_add(m[SIGMA[$r][2*$i+0] as usize]);
                $d = ($d ^ $a).rotate_right(16);
                $c = $c.wrapping_add($d);
                $b = ($b ^ $c).rotate_right(12);
                $a = $a.wrapping_add($b).wrapping_add(m[SIGMA[$r][2*$i+1] as usize]);
                $d = ($d ^ $a).rotate_right(8);
                $c = $c.wrapping_add($d);
                $b = ($b ^ $c).rotate_right(7);
            });
        );

        macro_rules! round(
            ($r: expr) => ({
                g!($r, 0, v[ 0], v[ 4], v[ 8], v[12]);
                g!($r, 1, v[ 1], v[ 5], v[ 9], v[13]);
                g!($r, 2, v[ 2], v[ 6], v[10], v[14]);
                g!($r, 3, v[ 3], v[ 7], v[11], v[15]);
                g!($r, 4, v[ 0], v[ 5], v[10], v[15]);
                g!($r, 5, v[ 1], v[ 6], v[11], v[12]);
                g!($r, 6, v[ 2], v[ 7], v[ 8], v[13]);
                g!($r, 7, v[ 3], v[ 4], v[ 9], v[14]);
            });
        );

        for i in 0..10 {
            round!(i);
        }

        for i in 0..8 {
            self.h[i] = self.h[i] ^ v[i] ^ v[i+8];
        }
    }
}

fn encode_params(size: u8, keylen: u8) -> [u8; 64] {
    let mut param = [0u8; 64];
    param[0] = size as u8;
    param[1] = keylen as u8;
    param[2] = 1; // fanout
    param[3] = 1; // depth
    param
}

fn load32(b: &[u8]) -> u32{
    let mut v = 0u32;
    for i in 0..4 {
        v |= (b[i] as u32) << (8*i); 
    }
    v
}

fn store32(b: &mut [u8], v: u32) {
    let mut w = v;
    for i in 0..4 {
        b[i] = w as u8;
        w >>= 8;
    }
}

#[cfg(test)]
mod tests {
    use super::{Blake2s, KEY_BYTES, OUT_BYTES};
    use super::super::kat;

    #[test]
    fn test_blake2s_out_size() {
        let input = [0u8; 256];

        for i in 0..kat::BLAKE2S_KAT_OUT_SIZE.len() {
            let out_size = i+1;
            let mut out = [0u8; OUT_BYTES];
            let mut h = Blake2s::new(out_size);
            h.update(input.as_ref());
            h.finalize(&mut out[..out_size]);
            assert_eq!(&out[..out_size], kat::BLAKE2S_KAT_OUT_SIZE[i]);
        }
    }

    #[test]
    fn test_blake2s_kat() {
        let mut input = [0u8; 256];
        for i in 0..input.len() {
            input[i] = i as u8;
        }

        for i in 0..kat::BLAKE2S_KAT.len() {
            let mut h = Blake2s::new(OUT_BYTES);
            let mut out = [0u8; OUT_BYTES];
            h.update(&input[..i]);
            h.finalize(&mut out);
            assert_eq!(out.as_ref(), kat::BLAKE2S_KAT[i].as_ref());
        }
    }

    #[test]
    fn test_blake2s_keyed_kat() {
        let mut input = [0u8; 256];
        let mut key = [0u8; KEY_BYTES];

        for i in 0..input.len() {
            input[i] = i as u8;
        }

        for i in 0..key.len() {
            key[i] = i as u8;
        }

        for i in 0..kat::BLAKE2S_KEYED_KAT.len() {
            let mut h = Blake2s::new_with_key(OUT_BYTES, key.as_ref());
            let mut out = [0u8; OUT_BYTES];
            h.update(&input[..i]);
            h.finalize(&mut out);
            assert_eq!(out.as_ref(), kat::BLAKE2S_KEYED_KAT[i].as_ref());
        }
    }
}

#[cfg(test)]
mod bench {
    use std::iter::repeat;

    use super::{Blake2s, OUT_BYTES};
    use test::Bencher;

    fn bench_chunk_size(b: &mut Bencher, n: usize) {
        let mut h = Blake2s::new(OUT_BYTES);
        let input : Vec<u8> = repeat(0).take(n).collect();
        b.bytes = input.len() as u64;
        b.iter(|| {
            h.update(input.as_ref());
        });
    }

    #[bench]
    fn bench_blake2s_16(b: &mut Bencher) {
        bench_chunk_size(b, 16);
    }

    #[bench]
    fn bench_blake2s_1k(b: &mut Bencher) {
        bench_chunk_size(b, 1 << 10);
    }

    #[bench]
    fn bench_blake2s_64k(b: &mut Bencher) {
        bench_chunk_size(b, 1 << 16);
    }
}
