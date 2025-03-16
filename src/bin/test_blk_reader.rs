
use std::time::SystemTime;

use num_format::{
    Locale,
    ToFormattedString,
};

use bitcoin_blk_reader::{
    block_to_block_hash,
    BlkReader,
};

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <rest_endpoint> <blocks_dir>", args[0]);
        std::process::exit(1);
    }
    let rest_endpoint = &args[1];
    let blocks_dir = &args[2];
    let mut blk_reader = BlkReader::new(rest_endpoint.clone(), blocks_dir.clone());
    let start_time = SystemTime::now();
    blk_reader.init(0).await;
    println!("Initialized in {}ms.", start_time.elapsed().unwrap().as_millis().to_formatted_string(&Locale::en));
    let start_time = SystemTime::now();
    for (height, block) in blk_reader {
        let mut block_hash = block_to_block_hash(&block);
        block_hash.reverse();
        println!("Height: {}, Block ID: {}", height.to_formatted_string(&Locale::en), hex::encode(block_hash));
    }
    println!("Fetched all blocks in {}ms.", start_time.elapsed().unwrap().as_millis().to_formatted_string(&Locale::en));
}
