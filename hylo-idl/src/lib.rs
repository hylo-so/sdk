anchor_lang::declare_program!(hylo_exchange);
anchor_lang::declare_program!(hylo_stability_pool);

pub mod instructions;
pub mod pda;
pub mod tokens;

const MPL_TOKEN_METADATA_ID: solana_pubkey::Pubkey =
  solana_pubkey::Pubkey::from_str_const(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
  );
