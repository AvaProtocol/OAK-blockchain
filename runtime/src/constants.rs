pub mod currency {
  use crate::Balance;
  pub const BDTS: Balance = 1_000_000_000_000;    // 12bits
  pub const DOLLARS: Balance= BDTS / 100;        // 10_000_000_000
  pub const CENTS: Balance = DOLLARS / 100;       // 100_000_000
  pub const MILLICENTS: Balance = CENTS / 1_000;  // 100_000

  pub const fn deposit(items: u32, bytes: u32) -> Balance {
      items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS
  }
}
