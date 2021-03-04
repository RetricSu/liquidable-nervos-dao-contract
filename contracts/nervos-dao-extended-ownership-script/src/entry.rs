// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
// use alloc::{vec, vec::Vec};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    debug,
    high_level::{load_script, load_cell_type_hash, load_cell_data, load_witness_args, load_tx_hash},
    ckb_types::{bytes::Bytes, prelude::*},
    ckb_constants::Source,
    error::SysError,
    dynamic_loading::CKBDLContext
};

use crate::error::Error;
use blake2b_ref::{Blake2b, Blake2bBuilder};
use ckb_lib_secp256k1::LibSecp256k1;

fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build()
}

pub fn main() -> Result<(), Error> {

    let mut total_puppet = 0;
    let mut puppet_id = 0;

    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    if args.is_empty() || args.len() != 32 {
        return Err(Error::NoValidArgs);
    };

    // debug!("start looking for puppet cell...");
    // let's find the puppet cell attched to nervosDAO cell
    for i in 0.. {
        match load_cell_type_hash(i, Source::Input){
            Ok(result) => {
                match result{
                    Some(type_hash) => {
                        if args[..] == type_hash[..] {
                            total_puppet = total_puppet + 1;
                            puppet_id = i;
                        };
                    },
                    None => {},
                };
            },
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };
    }

    // debug!("finished looking for puppet cell..., 
    //    found puppet id: {:?}, total_puppet: {:?}", 
    //    puppet_id, total_puppet
    // );

    match total_puppet {
        0 => return Err(Error::NoPuppetCellFound),
        n if n > 1 => return Err(Error::RequiredOnlyOnePuppet),
        _ => {},
    }

    // debug!("start to find public key hash...");

    let public_key_hash = match load_cell_data(puppet_id, Source::Input){
        Ok(data) => data,
        Err(SysError::IndexOutOfBound) => return Err(Error::NoPuppetCellFound),
        Err(err) => return Err(err.into()),
    };
    // debug!("public key is {:?} {:?}", public_key_hash, public_key_hash.pack());
    
    if public_key_hash.len() != 20 {
        return Err(Error::WrongPubkeyHashLength);
    }

    let witness_args = match load_witness_args(0, Source::GroupInput) {
        Ok(data) => data,
        Err(SysError::IndexOutOfBound) => return Err(Error::NoPuppetCellFound),
        Err(_err) => return Err(Error::LoadWitnessArgs),
    };
    // debug!("witness_args: {:?}", witness_args);

    let witness: Bytes = witness_args
            .lock()
            .to_opt()
            .ok_or(Error::WitnessArgsEncoding)?
            .unpack();
    
    // Validate the signatures of puppet cell
    // debug!("...start check signature with secp256");
    let mut context = unsafe{ CKBDLContext::<[u8; 128 * 1024]>::new()};
    let lib = LibSecp256k1::load(&mut context);
 
    // recover pubkey_hash
    let prefilled_data = lib.load_prefilled_data().map_err(|err| {
        debug!("load prefilled data error: {}", err);
        Error::LoadPrefilledData
    })?;

    let message:[u8; 32] = load_tx_hash().unwrap();
    let mut signature = [0u8; 65];
    let sig_len = signature.len();
    signature.copy_from_slice(&witness[0..sig_len]);
    
    let sign_pubkey = lib
            .recover_pubkey(&prefilled_data, &signature, &message)
            .map_err(|err| {
                debug!("recover pubkey error: {}", err);
                Error::RecoverPubkey
            })?;
    let sign_pubkey_hash = {
            let mut buf = [0u8; 32];
            let mut hasher = new_blake2b();
            hasher.update(sign_pubkey.as_slice());
            hasher.finalize(&mut buf);
            buf
        };
        
    if sign_pubkey_hash[..20] == public_key_hash[..] {
        return Ok(());
    }else{
        return Err(Error::WrongPublicKey);
    }
}

