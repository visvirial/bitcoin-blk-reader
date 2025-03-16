
use bitcoin_hashes::{
    Sha256d,
};

pub mod bitcoin_rest;
pub use bitcoin_rest::{
    BitcoinRest,
    BitcoinRestError,
};
pub mod blk_reader;
pub use blk_reader::{
    BlkReader,
};

pub fn block_to_block_hash(block: &[u8]) -> [u8; 32] {
    if block.len() < 80 {
        panic!("Block is too short.");
    }
    Sha256d::hash(&block[0..80]).to_byte_array()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_block_to_block_hash() {
        let block = hex::decode("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c").unwrap();
        let mut hash = block_to_block_hash(&block);
        hash.reverse();
        assert_eq!(hex::encode(hash), "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f");
    }
    
}
