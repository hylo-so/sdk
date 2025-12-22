use anchor_lang::prelude::*;

use crate::virtual_stablecoin::VirtualStablecoin;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct ExoPair {
  pub collateral_mint: Pubkey,
  pub virtual_stablecoin: VirtualStablecoin,
}
