use crypto_box::aead::generic_array::GenericArray;
use crypto_box::{aead::Aead, Box, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

pub fn get_pk_bytes(bytes: Vec<u8>) -> [u8; 32] {
    let mut bytes_vec = bytes.clone();
    if bytes.len() != 32 {
        let diff = 32 - bytes.len();
        let mut add = vec![0; diff];
        bytes_vec.append(&mut add);
    }
    bytes_vec.try_into().unwrap()
}
/*
 * ENCRYPTION STUFF
 */
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct EncData {
    pub encdata: Vec<u8>,
    pub nonce: Vec<u8>,
    pub pubkey: Vec<u8>,
}
pub fn decrypt_encdata(ed: &EncData, decrypt_cap: &Vec<u8>) -> (bool, Vec<u8>) {
    if decrypt_cap.is_empty() {
        return (false, vec![]);
    }

    //let start = time::Instant::now();
    let secretkey = SecretKey::from(get_pk_bytes(decrypt_cap.clone()));
    let pubkey = PublicKey::from(get_pk_bytes(ed.pubkey.clone()));
    let salsabox = Box::new(&pubkey, &secretkey);
    /*debug!(
        "decrypt {:?} with secret {} and pubkey {}",
        base64::encode(&ed.encdata),
        base64::encode(&decrypt_cap),
        base64::encode(&ed.pubkey),
    );*/
    match salsabox.decrypt(&GenericArray::from_slice(&ed.nonce), &ed.encdata[..]) {
        Ok(plaintext) => {
            /*debug!(
                "decrypted {}: {}",
                plaintext.len(),
                start.elapsed().as_micros()
            );*/
            (true, plaintext)
        }
        _ => (false, vec![]),
    }
}

pub fn encrypt_with_pubkey(pubkey: &PublicKey, bytes: &Vec<u8>) -> EncData {
    //let start = time::Instant::now();
    let mut rng = crypto_box::rand_core::OsRng;
    // this generates a new secret key each time
    let secretkey = SecretKey::generate(&mut rng);
    let edna_pubkey = PublicKey::from(&secretkey);
    let salsabox = Box::new(pubkey, &secretkey);
    let nonce = crypto_box::generate_nonce(&mut rng);
    let encrypted = salsabox.encrypt(&nonce, &bytes[..]).unwrap();
    /*debug!(
        "encrypt to {:?} with secret {} and pubkey {}, pair {}",
        base64::encode(&encrypted),
        base64::encode(&secretkey.to_bytes()),
        base64::encode(&pubkey.as_bytes()),
        base64::encode(&edna_pubkey.as_bytes()),
    );
    debug!("encrypt: {}", start.elapsed().as_micros());*/
    EncData {
        encdata: encrypted,
        nonce: nonce.to_vec(),
        pubkey: edna_pubkey.as_bytes().to_vec(),
    }
}
