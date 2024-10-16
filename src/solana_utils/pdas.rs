use super::constants::{BUBBLEGUM_PROGRAM_ID, TM_PROGRAM_ID};
use anchor_lang::solana_program::hash::hash;
use helium_anchor_gen::{helium_entity_manager, helium_sub_daos, iot_routing_manager};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn bubblegum_signer_key() -> Pubkey {
    let bubblegum_program =
        Pubkey::from_str(BUBBLEGUM_PROGRAM_ID).expect("Invalid BUBBLEGUM_PROGRAM_ID");

    Pubkey::find_program_address(&[b"collection_cpi"], &bubblegum_program).0
}

pub fn dao_key(hnt_mint: &Pubkey, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_sub_daos::ID);
    Pubkey::find_program_address(&[b"dao", hnt_mint.as_ref()], program_id).0
}

pub fn sub_dao_key(iot_mint: &Pubkey, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_sub_daos::ID);
    Pubkey::find_program_address(&[b"sub_dao", iot_mint.as_ref()], program_id).0
}

pub fn program_approval_key(
    dao: &Pubkey,
    irm_id: Option<&Pubkey>,
    program_id: Option<&Pubkey>,
) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_entity_manager::ID);
    let irm_id = irm_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(
        &[b"program_approval", dao.as_ref(), irm_id.as_ref()],
        program_id,
    )
    .0
}

pub fn key_to_asset_key_raw(
    dao: &Pubkey,
    hashed_entity_key: &[u8],
    program_id: Option<&Pubkey>,
) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_entity_manager::ID);
    Pubkey::find_program_address(
        &[b"key_to_asset", dao.as_ref(), hashed_entity_key],
        program_id,
    )
    .0
}

pub fn shared_merkle_key(proof_size: u8, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_entity_manager::ID);
    Pubkey::find_program_address(&[b"shared_merkle", &[proof_size]], program_id).0
}

pub fn routing_manager_key(sub_dao: &Pubkey, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(&[b"routing_manager", sub_dao.as_ref()], program_id).0
}

pub fn organization_key(routing_manager: &Pubkey, oui: u64, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(
        &[
            b"organization",
            routing_manager.as_ref(),
            &oui.to_le_bytes(),
        ],
        program_id,
    )
    .0
}

pub fn devaddr_constraint_key(
    organization: &Pubkey,
    start_addr: u64,
    program_id: Option<&Pubkey>,
) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(
        &[
            b"devaddr_constraint",
            organization.as_ref(),
            &start_addr.to_le_bytes(),
        ],
        program_id,
    )
    .0
}

pub fn net_id_key(routing_manager: &Pubkey, id: u64, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(
        &[b"net_id", routing_manager.as_ref(), &id.to_le_bytes()],
        program_id,
    )
    .0
}

pub fn organization_delegate_key(
    organization: &Pubkey,
    delegate: &Pubkey,
    program_id: Option<&Pubkey>,
) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(
        &[
            b"organization_delegate",
            organization.as_ref(),
            delegate.as_ref(),
        ],
        program_id,
    )
    .0
}

pub fn routing_manager_collection_key(
    routing_manager: &Pubkey,
    program_id: Option<&Pubkey>,
) -> Pubkey {
    let program_id = program_id.unwrap_or(&iot_routing_manager::ID);
    Pubkey::find_program_address(&[b"collection", routing_manager.as_ref()], program_id).0
}

pub fn routing_manager_collection_metadata_key(collection: &Pubkey) -> Pubkey {
    let token_metadata_program = Pubkey::from_str(TM_PROGRAM_ID).expect("Invalid TM_PROGRAM_ID");
    Pubkey::find_program_address(
        &[
            b"metadata",
            token_metadata_program.as_ref(),
            collection.as_ref(),
        ],
        &token_metadata_program,
    )
    .0
}

pub fn routing_manager_collection_master_edition_key(collection: &Pubkey) -> Pubkey {
    let token_metadata_program = Pubkey::from_str(TM_PROGRAM_ID).expect("Invalid TM_PROGRAM_ID");
    Pubkey::find_program_address(
        &[
            b"metadata",
            token_metadata_program.as_ref(),
            collection.as_ref(),
            b"edition",
        ],
        &token_metadata_program,
    )
    .0
}

pub fn entity_creator_key(dao: &Pubkey, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_entity_manager::ID);
    Pubkey::find_program_address(&[b"entity_creator", dao.as_ref()], program_id).0
}

pub fn key_to_asset_key(dao: &Pubkey, oui: u64, program_id: Option<&Pubkey>) -> Pubkey {
    let program_id = program_id.unwrap_or(&helium_entity_manager::ID);
    let seed_str = format!("OUI_{}", oui);
    let hashed_entity = hash(seed_str.as_bytes()).to_bytes();

    Pubkey::find_program_address(&[b"key_to_asset", dao.as_ref(), &hashed_entity], program_id).0
}

pub fn tree_authority_key(merkle_tree: &Pubkey) -> Pubkey {
    let bubblegum_program =
        Pubkey::from_str(BUBBLEGUM_PROGRAM_ID).expect("Invalid BUBBLEGUM_PROGRAM_ID");

    Pubkey::find_program_address(&[merkle_tree.as_ref()], &bubblegum_program).0
}
