use ckb_tool::ckb_crypto::secp::{Generator, Privkey, Pubkey};
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{TransactionView},
    packed::{self, *},
    prelude::*,
    H256,
};
use ckb_tool::ckb_hash::{blake2b_256};

pub fn blake160(data: &[u8]) -> [u8; 20] {
    let mut buf = [0u8; 20];
    let hash = blake2b_256(data);
    buf.clone_from_slice(&hash[..20]);
    buf
}

pub fn generate_key_pair() -> (Privkey, Pubkey) {
    return Generator::random_keypair();
}


pub fn prepare_witnesses(tx: TransactionView, index: usize, key: &Privkey) -> TransactionView {
    const SIGNATURE_SIZE: usize = 65;
    
    let tx_hash = tx.hash();
    let message: [u8; 32] = tx_hash.clone().unpack();
    let message = H256::from(message);
    let sig = key.sign_recoverable(&message).expect("sign");

    let witnesses_len = tx.inputs().len();

    let witness = WitnessArgs::default();
    let mut signed_witnesses: Vec<packed::Bytes> = Vec::new();
    
    for i in 0..witnesses_len {
        if i == index {
            signed_witnesses.push(
                witness.clone()
                    .as_builder()
                    .lock(Some(Bytes::from(sig.serialize())).pack())
                    .build()
                    .as_bytes()
                    .pack(),
            );
        }else {
            // push empty witness in other field
            let zero_lock: Bytes = {
                let mut buf = Vec::new();
                buf.resize(SIGNATURE_SIZE, 0);
                buf.into()
            };
            let witness_for_digest = witness
                .clone()
                .as_builder()
                .lock(Some(zero_lock).pack())
                .build()
                .as_bytes()
                .pack();
            signed_witnesses.push(witness_for_digest);
        }
    }
    
    tx.as_advanced_builder()
        .set_witnesses(signed_witnesses)
        .build()
}
