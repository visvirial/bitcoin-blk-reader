
use std::time::SystemTime;
use std::fs::File;
use std::io::Write;
use std::io::BufWriter;

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
    if args.len() <= 4 {
        eprintln!("Usage: {} <rest_endpoint> <blocks_dir> <bootstrap.dat> [<start_height> [<end_height>]]", args[0]);
        std::process::exit(1);
    }
    let rest_endpoint = &args[1];
    let blocks_dir = &args[2];
    let outpath = &args[3];
    let start_height = if args.len() > 4 {
        args[4].parse::<u32>().unwrap()
    } else {
        0
    };
    let end_height = if args.len() > 5 {
        args[5].parse::<u32>().unwrap()
    } else {
        u32::MAX
    };
    let outfile = File::create(outpath).unwrap();
    let mut writer = BufWriter::new(outfile);
    let mut blk_reader = BlkReader::new(rest_endpoint.clone(), blocks_dir.clone());
    let start_time = SystemTime::now();
    blk_reader.init(start_height).await.unwrap();
    eprintln!("Initialized in {}ms.", start_time.elapsed().unwrap().as_millis().to_formatted_string(&Locale::en));
    let start_time = SystemTime::now();
    for (height, block, magic) in blk_reader {
        // Write magic bytes.
        writer.write_all(&magic).unwrap();
        // Write block size.
        writer.write_all(&(block.len() as u32).to_le_bytes()).unwrap();
        // Write block body.
        writer.write_all(&block).unwrap();
        let mut block_hash = block_to_block_hash(&block);
        block_hash.reverse();
        eprintln!("Height: {}, Block ID: {}", height.to_formatted_string(&Locale::en), hex::encode(block_hash));
        if height >= end_height {
            break;
        }
    }
    eprintln!("All blocks written in {}ms.", start_time.elapsed().unwrap().as_millis().to_formatted_string(&Locale::en));
}
