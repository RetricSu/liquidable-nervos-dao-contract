use super::*;
use ckb_testtool::{builtin::ALWAYS_SUCCESS, context::Context};
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::TransactionBuilder,
    packed::*,
    prelude::*,
};
use ckb_tool::ckb_error::assert_error_eq;
use ckb_tool::ckb_script::ScriptError;
use ckb_system_scripts::BUNDLED_CELL;

use std::println;

use helper;

const MAX_CYCLES: u64 = 10_000_000;

// error numbers
const ERROR_EMPTY_ARGS: i8 = 5;
const ERROR_NO_PUPPET_CELL: i8 = 6;
const ERROR_ONLY_ONE_PUPPET_CELL: i8 = 7;
const ERROR_WRONG_PUBKEY: i8 = 10;
const ERROR_WRONG_PUBKEY_HASH_LEN: i8 = 11;

#[test]
fn test_with_empty_args() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&out_point, Default::default())
        .expect("script");
    let lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(lock_script_dep)
        .build();
    let tx = context.complete_tx(tx);

    // run
    let err = context
        .verify_tx(&tx, MAX_CYCLES)
        .unwrap_err();
    // we expect an error raised from 0-indexed cell's lock script
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ERROR_EMPTY_ARGS).input_lock_script(script_cell_index)
    );
}

#[test]
fn test_with_empty_pubkey() {

    let (privkey, pubkey) = helper::generate_key_pair();

    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // deploy secp256 contract
    let secp256k1_bin: Bytes =
        fs::read("../ckb-miscellaneous-scripts/build/secp256k1_blake2b_sighash_all_dual")
            .expect("load secp256k1")
            .into();
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);
    let secp256k1_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    

    // build always success lock script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let as_lock_script = context
        .build_script(&(&always_success_out_point), Default::default())
        .expect("script");

    // # build a faked puppet id type scirpt
    // here we just using the always success script as a type-id script
    // but we pass a unique args after-all
    let puppet_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x11000000".to_string()))
        .expect("script");
    
    // # build a faked nervos dao type scirpt
    // here we just using the always success script as a dao script
    // but we pass a unique args after-all
    let dao_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x0011000000".to_string()))
        .expect("script");

    
    let as_lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();


    // create puppet cell
    let puppet_cell_out_point = context.create_cell(
         CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(as_lock_script.clone())
            .type_(Some(puppet_type_script.clone()).pack())
            .build(),
        Bytes::new(), // here we place wrong publickey
    );

    // prepare nervos-dao cell
    // here we set wrong type script on popurse
    let dao_ownership_id: [u8; 32] = puppet_type_script.clone().calc_script_hash().unpack();
    let dao_ownership_lock_args: Bytes = dao_ownership_id.to_vec().into();
    let dao_ownership_lock_script = context
        .build_script(&out_point, dao_ownership_lock_args)
        .expect("script");
    let dao_ownership_lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    let dao_cell_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(dao_ownership_lock_script.clone())
            .type_(Some(dao_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );

    // prepare tx's inputs and outputs
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(dao_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point)
            .build()
    ];
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_ownership_lock_script_dep)
        .cell_dep(as_lock_script_dep)
        .cell_dep(secp256k1_dep)
        .cell_dep(secp256k1_data_dep)
        .witnesses(witnesses.pack())
        .build();
        
    let signed_tx = context.complete_tx(tx);
    let signed_tx = helper::prepare_witnesses(signed_tx, 0, &privkey.clone());
    
    // run
    let err = context
    .verify_tx(&signed_tx, MAX_CYCLES)
    .unwrap_err();
    // we expect an error raised from 0-indexed cell's lock script
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ERROR_WRONG_PUBKEY_HASH_LEN).input_lock_script(script_cell_index)
    );
}

#[test]
fn test_with_two_puppect_cells() {

    let (privkey, pubkey) = helper::generate_key_pair();
    let pubkey_hash = helper::blake160(&pubkey.serialize());

    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // deploy secp256 contract
    let secp256k1_bin: Bytes =
        fs::read("../ckb-miscellaneous-scripts/build/secp256k1_blake2b_sighash_all_dual")
            .expect("load secp256k1")
            .into();
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);
    let secp256k1_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    

    // build always success lock script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let as_lock_script = context
        .build_script(&(&always_success_out_point), Default::default())
        .expect("script");

    // # build a faked puppet id type scirpt
    // here we just using the always success script as a type-id script
    // but we pass a unique args after-all
    let puppet_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x11000000".to_string()))
        .expect("script");
    
    // # build a faked nervos dao type scirpt
    // here we just using the always success script as a dao script
    // but we pass a unique args after-all
    let dao_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x0011000000".to_string()))
        .expect("script");

    let as_lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();


    // create puppet cell
    let puppet_cell_out_point = context.create_cell(
         CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(as_lock_script.clone())
            .type_(Some(puppet_type_script.clone()).pack())
            .build(),
        pubkey_hash.clone().to_vec().into(), 
    );

    //create second puppet cell
    let puppet_cell_out_point_2 = context.create_cell(
        CellOutput::new_builder()
           .capacity(500u64.pack())
           .lock(as_lock_script.clone())
           .type_(Some(puppet_type_script.clone()).pack())
           .build(),
       pubkey_hash.clone().to_vec().into(), 
   ); 

    // prepare nervos-dao cell
    // here we set wrong type script on popurse
    let dao_ownership_id: [u8; 32] = puppet_type_script.clone().calc_script_hash().unpack();
    let dao_ownership_lock_args: Bytes = dao_ownership_id.to_vec().into();
    let dao_ownership_lock_script = context
        .build_script(&out_point, dao_ownership_lock_args)
        .expect("script");
    let dao_ownership_lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    let dao_cell_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(dao_ownership_lock_script.clone())
            .type_(Some(dao_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );

    // prepare tx's inputs and outputs
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(dao_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point_2)
            .build()
    ];
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_ownership_lock_script_dep)
        .cell_dep(as_lock_script_dep)
        .cell_dep(secp256k1_dep)
        .cell_dep(secp256k1_data_dep)
        .witnesses(witnesses.pack())
        .build();
    
    let signed_tx = context.complete_tx(tx);
    let signed_tx = helper::prepare_witnesses(signed_tx, 0, &privkey.clone());
    
    // run
    let err = context
    .verify_tx(&signed_tx, MAX_CYCLES)
    .unwrap_err();
    // we expect an error raised from 0-indexed cell's lock script
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ERROR_ONLY_ONE_PUPPET_CELL).input_lock_script(script_cell_index)
    );
}

#[test]
fn test_with_wrong_puppet_cell() {

    let (privkey, pubkey) = helper::generate_key_pair();
    let pubkey_hash = helper::blake160(&pubkey.serialize());

    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // deploy secp256 contract
    let secp256k1_bin: Bytes =
        fs::read("../ckb-miscellaneous-scripts/build/secp256k1_blake2b_sighash_all_dual")
            .expect("load secp256k1")
            .into();
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);
    let secp256k1_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    

    // build always success lock script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let as_lock_script = context
        .build_script(&(&always_success_out_point), Default::default())
        .expect("script");

    // # build a faked puppet id type scirpt
    // here we just using the always success script as a type-id script
    // but we pass a unique args after-all
    let puppet_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x11000000".to_string()))
        .expect("script");
    
    // # build a faked nervos dao type scirpt
    // here we just using the always success script as a dao script
    // but we pass a unique args after-all
    let dao_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x0011000000".to_string()))
        .expect("script");

    let wrong_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x10000000".to_string()))
        .expect("script");

    
    let as_lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();


    // create puppet cell
    let puppet_cell_out_point = context.create_cell(
         CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(as_lock_script.clone())
            .type_(Some(puppet_type_script.clone()).pack())
            .build(),
        pubkey_hash.to_vec().into(),//pubkey_hash.to_vec().into(),//pubkey.clone().as_bytes().pack().unpack(),
    );

    // prepare nervos-dao cell
    // here we set wrong type script on popurse
    let dao_ownership_id: [u8; 32] = wrong_type_script.clone().calc_script_hash().unpack();//puppet_type_script.clone().calc_script_hash().unpack();
    let dao_ownership_lock_args: Bytes = dao_ownership_id.to_vec().into();
    let dao_ownership_lock_script = context
        .build_script(&out_point, dao_ownership_lock_args)
        .expect("script");
    let dao_ownership_lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    let dao_cell_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(dao_ownership_lock_script.clone())
            .type_(Some(dao_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );

    // prepare tx's inputs and outputs
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(dao_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point)
            .build()
    ];
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_ownership_lock_script_dep)
        .cell_dep(as_lock_script_dep)
        .cell_dep(secp256k1_dep)
        .cell_dep(secp256k1_data_dep)
        .witnesses(witnesses.pack())
        .build();
        
    let signed_tx = context.complete_tx(tx);
    let signed_tx = helper::prepare_witnesses(signed_tx, 0, &privkey.clone());
    
    // run
    let err = context
    .verify_tx(&signed_tx, MAX_CYCLES)
    .unwrap_err();
    // we expect an error raised from 0-indexed cell's lock script
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ERROR_NO_PUPPET_CELL).input_lock_script(script_cell_index)
    );
}

#[test]
fn test_with_wrong_pubkey() {

    let (privkey, pubkey) = helper::generate_key_pair();

    let (wrong_privkey, wrong_pubkey) = helper::generate_key_pair();
    let wrong_pubkey_hash = helper::blake160(&wrong_pubkey.serialize());
    

    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // deploy secp256 contract
    let secp256k1_bin: Bytes =
        fs::read("../ckb-miscellaneous-scripts/build/secp256k1_blake2b_sighash_all_dual")
            .expect("load secp256k1")
            .into();
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);
    let secp256k1_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    

    // build always success lock script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let as_lock_script = context
        .build_script(&(&always_success_out_point), Default::default())
        .expect("script");

    // # build a faked puppet id type scirpt
    // here we just using the always success script as a type-id script
    // but we pass a unique args after-all
    let puppet_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x11000000".to_string()))
        .expect("script");
    
    // # build a faked nervos dao type scirpt
    // here we just using the always success script as a dao script
    // but we pass a unique args after-all
    let dao_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x0011000000".to_string()))
        .expect("script");

    
    let as_lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();


    // create puppet cell
    let puppet_cell_out_point = context.create_cell(
         CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(as_lock_script.clone())
            .type_(Some(puppet_type_script.clone()).pack())
            .build(),
        wrong_pubkey_hash.to_vec().into(), // here we place wrong publickey
    );

    // prepare nervos-dao cell
    // here we set wrong type script on popurse
    let dao_ownership_id: [u8; 32] = puppet_type_script.clone().calc_script_hash().unpack();
    let dao_ownership_lock_args: Bytes = dao_ownership_id.to_vec().into();
    let dao_ownership_lock_script = context
        .build_script(&out_point, dao_ownership_lock_args)
        .expect("script");
    let dao_ownership_lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    let dao_cell_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(dao_ownership_lock_script.clone())
            .type_(Some(dao_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );

    // prepare tx's inputs and outputs
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(dao_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point)
            .build()
    ];
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_ownership_lock_script_dep)
        .cell_dep(as_lock_script_dep)
        .cell_dep(secp256k1_dep)
        .cell_dep(secp256k1_data_dep)
        .witnesses(witnesses.pack())
        .build();
        
    let signed_tx = context.complete_tx(tx);
    let signed_tx = helper::prepare_witnesses(signed_tx, 0, &privkey.clone());
    
    // run
    let err = context
    .verify_tx(&signed_tx, MAX_CYCLES)
    .unwrap_err();
    // we expect an error raised from 0-indexed cell's lock script
    let script_cell_index = 0;
    assert_error_eq!(
        err,
        ScriptError::ValidationFailure(ERROR_WRONG_PUBKEY).input_lock_script(script_cell_index)
    );
}

#[test]
fn test_with_correct_tx() {

    let (privkey, pubkey) = helper::generate_key_pair();
    let pubkey_hash = helper::blake160(&pubkey.serialize());

    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("nervos-dao-extended-ownership-script");
    let out_point = context.deploy_cell(contract_bin);

    // deploy secp256 contract
    let secp256k1_bin: Bytes =
        fs::read("../ckb-miscellaneous-scripts/build/secp256k1_blake2b_sighash_all_dual")
            .expect("load secp256k1")
            .into();
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);
    let secp256k1_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();
    let secp256k1_data_bin = BUNDLED_CELL.get("specs/cells/secp256k1_data").unwrap();
    let secp256k1_data_out_point = context.deploy_cell(secp256k1_data_bin.to_vec().into());
    let secp256k1_data_dep = CellDep::new_builder()
        .out_point(secp256k1_data_out_point)
        .build();
    

    // build always success lock script
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let as_lock_script = context
        .build_script(&(&always_success_out_point), Default::default())
        .expect("script");

    // # build a faked puppet id type scirpt
    // here we just using the always success script as a type-id script
    // but we pass a unique args after-all
    let puppet_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x11000000".to_string()))
        .expect("script");
    
    // # build a faked nervos dao type scirpt
    // here we just using the always success script as a dao script
    // but we pass a unique args after-all
    let dao_type_script = context
        .build_script(&(&always_success_out_point), Bytes::from("0x0011000000".to_string()))
        .expect("script");


    let as_lock_script_dep = CellDep::new_builder()
        .out_point(always_success_out_point)
        .build();

    // create puppet cell
    let puppet_cell_out_point = context.create_cell(
         CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(as_lock_script.clone())
            .type_(Some(puppet_type_script.clone()).pack())
            .build(),
        pubkey_hash.to_vec().into(),//pubkey_hash.to_vec().into(),//pubkey.clone().as_bytes().pack().unpack(),
    );

    // prepare nervos-dao cell
    let dao_ownership_id: [u8; 32] = puppet_type_script.clone().calc_script_hash().unpack();
    let dao_ownership_lock_args: Bytes = dao_ownership_id.to_vec().into();
    let dao_ownership_lock_script = context
        .build_script(&out_point, dao_ownership_lock_args)
        .expect("script");
    let dao_ownership_lock_script_dep = CellDep::new_builder()
        .out_point(out_point)
        .build();

    let dao_cell_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(dao_ownership_lock_script.clone())
            .type_(Some(dao_type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );


    // prepare tx's inputs and outputs
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(dao_cell_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(puppet_cell_out_point)
            .build()
    ];
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(as_lock_script)
            .build(),
    ];

    let outputs_data = vec![Bytes::new(); 2];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(dao_ownership_lock_script_dep)
        .cell_dep(as_lock_script_dep)
        .cell_dep(secp256k1_dep)
        .cell_dep(secp256k1_data_dep)
        .witnesses(witnesses.pack())
        .build();
        
    let signed_tx = context.complete_tx(tx);
    let signed_tx = helper::prepare_witnesses(signed_tx, 0, &privkey.clone());
    
    // run
    let cycles = context
        .verify_tx(&signed_tx, MAX_CYCLES)
        .expect("pass verification");
    // println!("consume cycles: {}", cycles);   
}

