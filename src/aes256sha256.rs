use std::io::{Read, Seek, Write};

#[cfg(feature = "compress")]
pub use self::enc::*;
use crate::Password;
use aes::cipher::{generic_array::GenericArray, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use lzma_rust::CountingWriter;
use rand::Rng;
use sha2::Digest;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub struct Aes256Sha256Decoder<R> {
    cipher: Cipher,
    input: R,
    done: bool,
    obuffer: Vec<u8>,
    ostart: usize,
    ofinish: usize,
    pos: usize,
}

impl<R: Read> Aes256Sha256Decoder<R> {
    pub fn new(input: R, properties: &[u8], password: &[u8]) -> Result<Self, crate::Error> {
        let cipher = Cipher::from_properties(properties, password)?;
        Ok(Self {
            input,
            cipher,
            done: false,
            obuffer: Default::default(),
            ostart: 0,
            ofinish: 0,
            pos: 0,
        })
    }

    fn get_more_data(&mut self) -> std::io::Result<usize> {
        if self.done {
            Ok(0)
        } else {
            self.ofinish = 0;
            self.ostart = 0;
            self.obuffer.clear();
            let mut ibuffer = [0; 512];
            let readin = self.input.read(&mut ibuffer)?;
            if readin == 0 {
                self.done = true;
                self.ofinish = self.cipher.do_final(&mut self.obuffer)?;
                Ok(self.ofinish)
            } else {
                let n = self
                    .cipher
                    .update(&mut ibuffer[..readin], &mut self.obuffer)?;
                self.ofinish = n;
                Ok(n)
            }
        }
    }
}

impl<R: Read> Read for Aes256Sha256Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.ostart >= self.ofinish {
            let mut n: usize;
            n = self.get_more_data()?;
            while n == 0 && !self.done {
                n = self.get_more_data()?;
            }
            if n == 0 {
                return Ok(0);
            }
        }

        if buf.is_empty() {
            return Ok(0);
        }
        let buf_len = self.ofinish - self.ostart;
        let size = buf_len.min(buf.len());
        buf[..size].copy_from_slice(&self.obuffer[self.ostart..self.ostart + size]);
        self.ostart += size;
        self.pos += size;
        Ok(size)
    }
}

impl<R: Read + Seek> Seek for Aes256Sha256Decoder<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let len = self.ofinish - self.ostart;
        match pos {
            std::io::SeekFrom::Start(p) => {
                let n = (p as i64 - self.pos as i64).min(len as i64);

                if n < 0 {
                    Ok(0)
                } else {
                    self.ostart += n as usize;
                    Ok(p)
                }
            }
            std::io::SeekFrom::End(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Aes256 decoder unsupport seek from end",
            )),
            std::io::SeekFrom::Current(n) => {
                let n = n.min(len as i64);
                if n < 0 {
                    Ok(0)
                } else {
                    self.ostart += n as usize;
                    Ok(self.pos as u64 + n as u64)
                }
            }
        }
    }
}

fn get_aes_key(properties: &[u8], password: &[u8]) -> Result<([u8; 32], [u8; 16]), crate::Error> {
    if properties.len() < 2 {
        return Err(crate::Error::other("AES256 properties too shart"));
    }
    let b0 = properties[0];
    let num_cycles_power = b0 & 63;
    let b1 = properties[1];
    let iv_size = ((b0 >> 6 & 1) + (b1 & 15)) as usize;
    let salt_size = ((b0 >> 7 & 1) + (b1 >> 4)) as usize;
    if 2 + salt_size + iv_size > properties.len() {
        return Err(crate::Error::other("Salt size + IV size too long"));
    }
    let mut salt = vec![0u8; salt_size];
    salt.copy_from_slice(&properties[2..(2 + salt_size)]);
    let mut iv = [0u8; 16];
    iv[0..iv_size].copy_from_slice(&properties[(2 + salt_size)..(2 + salt_size + iv_size)]);
    if password.is_empty() {
        return Err(crate::Error::PasswordRequired);
    }
    let aes_key = if num_cycles_power == 0x3f {
        let mut aes_key = [0u8; 32];
        aes_key.copy_from_slice(&salt[..salt_size]);
        let n = password.len().min(aes_key.len() - salt_size);
        aes_key[salt_size..n + salt_size].copy_from_slice(&password[0..n]);
        aes_key
    } else {
        let mut sha = sha2::Sha256::default();
        let mut extra = [0u8; 8];
        for _ in 0..(1u32 << num_cycles_power) {
            sha.update(&salt);
            sha.update(password);
            sha.update(extra);
            for item in &mut extra {
                *item = item.wrapping_add(1);
                if *item != 0 {
                    break;
                }
            }
        }
        sha.finalize().into()
    };
    Ok((aes_key, iv))
}

struct Cipher {
    dec: Aes256CbcDec,
    buf: Vec<u8>,
}

impl Cipher {
    fn from_properties(properties: &[u8], password: &[u8]) -> Result<Self, crate::Error> {
        let (aes_key, iv) = get_aes_key(properties, password)?;
        Ok(Self {
            dec: Aes256CbcDec::new(&GenericArray::from(aes_key), &iv.into()),
            buf: Default::default(),
        })
    }

    fn update<W: std::io::Write>(
        &mut self,
        mut data: &mut [u8],
        mut output: W,
    ) -> std::io::Result<usize> {
        let mut n = 0;
        if !self.buf.is_empty() {
            assert!(self.buf.len() < 16);
            let end = 16 - self.buf.len();
            self.buf.extend_from_slice(&data[..end]);
            data = &mut data[end..];
            let block = GenericArray::from_mut_slice(&mut self.buf);
            self.dec.decrypt_block_mut(block);
            let out = block.as_slice();
            output.write_all(out)?;
            n += out.len();
            self.buf.clear();
        }

        for a in data.chunks_mut(16) {
            if a.len() < 16 {
                self.buf.extend_from_slice(a);
                break;
            }
            let block = GenericArray::from_mut_slice(a);
            self.dec.decrypt_block_mut(block);
            let out = block.as_slice();
            output.write_all(out)?;
            n += out.len();
        }
        Ok(n)
    }

    fn do_final(&mut self, output: &mut Vec<u8>) -> std::io::Result<usize> {
        if self.buf.is_empty() {
            output.clear();
            Ok(0)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "IllegalBlockSize",
            ))
        }
    }
}
#[cfg(feature = "compress")]
mod enc {
    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
    use super::*;
    pub struct Aes256Sha256Encoder<W> {
        output: CountingWriter<W>,
        enc: Aes256CbcEnc,
        buffer: Vec<u8>,
        done: bool,
        write_size: u32,
    }
    #[derive(Debug, Clone)]
    pub struct AesEncoderOptions {
        pub password: Password,
        pub iv: [u8; 16],
        pub salt: [u8; 16],
        pub num_cycles_power: u8,
    }

    impl AesEncoderOptions {
        pub fn new(password: Password) -> Self {
            fn random_arr() -> [u8; 16] {
                let mut a = [0u8; 16];
                let mut r = rand::thread_rng();
                for i in a.iter_mut() {
                    *i = r.gen();
                }
                a
            }
            Self {
                password,
                iv: random_arr(),
                salt: random_arr(),
                num_cycles_power: 8,
            }
        }

        pub fn properties(&self) -> [u8; 34] {
            let mut props = [0u8; 34];
            self.write_properties(&mut props);
            props
        }

        #[inline]
        pub fn write_properties(&self, props: &mut [u8]) {
            assert!(props.len() >= 34);
            props[0] = (self.num_cycles_power & 0x3f) | 0xc0;
            props[1] = 0xff;
            props[2..18].copy_from_slice(&self.salt);
            props[18..34].copy_from_slice(&self.iv);
        }
    }

    impl<W> Aes256Sha256Encoder<W> {
        pub fn new(
            output: CountingWriter<W>,
            options: &AesEncoderOptions,
        ) -> Result<Self, crate::Error> {
            let (key, iv) = get_aes_key(&options.properties(), options.password.as_slice())?;

            Ok(Self {
                output,
                enc: Aes256CbcEnc::new(&GenericArray::from(key), &iv.into()),
                buffer: Default::default(),
                done: false,
                write_size: 0,
            })
        }

        #[inline(always)]
        fn write_block(&mut self, block: &mut [u8]) -> std::io::Result<()>
        where
            W: Write,
        {
            let block2 = GenericArray::from_mut_slice(block);
            self.enc.encrypt_block_mut(block2);
            self.output.write_all(block)?;
            self.write_size += block.len() as u32;
            Ok(())
        }
    }

    impl<W: Write> Write for Aes256Sha256Encoder<W> {
        fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
            if self.done && !buf.is_empty() {
                return Ok(0);
            }
            if buf.is_empty() {
                self.done = true;
                self.flush()?;
                return self.output.write(buf);
            }
            let len = buf.len();
            if !self.buffer.is_empty() {
                assert!(self.buffer.len() < 16);
                if buf.len() + self.buffer.len() >= 16 {
                    let buffer = &self.buffer[..];
                    let end = 16 - buffer.len();

                    let mut block = [0u8; 16];
                    block[0..buffer.len()].copy_from_slice(buffer);
                    block[buffer.len()..16].copy_from_slice(&buf[..end]);
                    // let block2 = GenericArray::from_mut_slice(&mut block);
                    // self.enc.encrypt_block_mut(block2);
                    // self.output.write_all(&block)?;
                    // self.write_size += block.len() as _;
                    self.write_block(&mut block)?;
                    self.buffer.clear();
                    buf = &buf[end..];
                } else {
                    // self.buffer.drain(..start);
                    self.buffer.extend_from_slice(buf);
                    return Ok(len);
                }
            }

            for data in buf.chunks(16) {
                if data.len() < 16 {
                    self.buffer.extend_from_slice(data);
                    break;
                }
                let mut block = [0u8; 16];
                block.copy_from_slice(data);
                self.write_block(&mut block)?;
            }

            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            if !self.buffer.is_empty() && self.done {
                assert!(self.buffer.len() < 16);
                let mut block = [0u8; 16];
                block[..self.buffer.len()].copy_from_slice(&self.buffer);
                self.write_block(&mut block)?;
                self.buffer.clear();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "compress")]
    #[test]
    fn test_aes_codec() {
        let mut encoded = vec![];
        let mut writer = CountingWriter::new(&mut encoded);
        let pwd: Password = "1234".into();
        let options = AesEncoderOptions::new(pwd.clone());
        let mut enc = Aes256Sha256Encoder::new(writer, &options).unwrap();
        let original = include_bytes!("./aes256sha256.rs");
        enc.write_all(original).expect("encode data");
        enc.write(&[]);

        let mut encoded_data = &encoded[..];
        let mut dec =
            Aes256Sha256Decoder::new(&mut encoded_data, &options.properties(), pwd.as_slice())
                .unwrap();

        let mut decoded = vec![];
        std::io::copy(&mut dec, &mut decoded);
        assert_eq!(&decoded[..original.len()], &original[..]);
    }
}
