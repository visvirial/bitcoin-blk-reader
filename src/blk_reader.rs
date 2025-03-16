
use std::io::prelude::*;
use std::io::BufReader;
//use std::time::SystemTime;
use std::fs::File;
use std::sync::{
    Arc,
    RwLock,
};
use std::collections::{
    HashMap,
};
use bytes::Bytes;

use crate::{
    block_to_block_hash,
    BitcoinRest,
};

#[derive(Debug, Clone)]
pub struct BlkReaderData {
    // Block height -> block,
    blocks: HashMap<u32, Bytes>,
    block_height_by_hash: HashMap<[u8; 32], u32>,
    next_blk_index: u32,
    next_height: u32,
    all_read: bool,
}

impl BlkReaderData {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            block_height_by_hash: HashMap::new(),
            next_blk_index: 0,
            next_height: 0,
            all_read: false,
        }
    }
}

#[derive(Debug)]
pub struct BlkFileReader {
    reader: BufReader<File>,
    xor: [u8; 8],
    position: u64,
}

impl BlkFileReader {
    pub fn new(path: &str, xor: [u8; 8]) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            reader,
            xor,
            position: 0,
        })
    }
}

impl Read for BlkFileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.reader.read(buf)?;
        for i in 0..read {
            buf[i] ^= self.xor[(self.position % 8) as usize];
            self.position += 1;
        }
        Ok(read)
    }
}

#[derive(Debug, Clone)]
pub struct BlkReader {
    rest_endpoint: String,
    blocks_dir: String,
    end_height: u32,
    xor: [u8; 8],
    data: Arc<RwLock<BlkReaderData>>,
}

impl BlkReader {
    pub fn new(rest_endpoint: String, blocks_dir: String) -> Self {
        Self {
            rest_endpoint,
            blocks_dir,
            end_height: 0,
            xor: [0u8; 8],
            data: Arc::new(RwLock::new(BlkReaderData::new())),
        }
    }
    pub async fn init(&mut self, starting_height: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.xor = self.read_xor()?;
        //println!("XOR: {}", hex::encode(self.xor));
        let bitcoin_rest = BitcoinRest::new(self.rest_endpoint.clone());
        // Get starting block hash.
        let start_block_hash = bitcoin_rest.get_blockhashbyheight(starting_height).await?;
        //println!("Starting block hash: {}", hex::encode(start_block_hash));
        // Download all block headers.
        //println!("Fetching all block headers...");
        //let start_time = SystemTime::now();
        let headers = bitcoin_rest.get_all_headers(start_block_hash, None).await?;
        //let blocks_len = headers.len();
        //println!("Fetched {} block headers in {}ms.", blocks_len, start_time.elapsed().unwrap().as_millis());
        // Convert to block_height_by_hash.
        for (offset, header) in headers.iter().enumerate() {
            let block_hash = block_to_block_hash(header);
            let height = starting_height + offset as u32;
            self.data.write().unwrap().block_height_by_hash.insert(block_hash, height);
        }
        self.data.write().unwrap().next_height = starting_height;
        self.end_height = starting_height + headers.len() as u32 - 1;
        Ok(())
    }
    pub fn is_all_read(&self) -> bool {
        self.data.read().unwrap().all_read
    }
    pub fn get_registered_block_count(&self) -> usize {
        self.data.read().unwrap().blocks.len()
    }
    pub fn get_next_height(&self) -> u32 {
        self.data.read().unwrap().next_height
    }
    pub fn read_xor(&self) -> Result<[u8; 8], std::io::Error> {
        let path = format!("{}/xor.dat", self.blocks_dir);
        let file = File::open(&path);
        if let Err(e) = file {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok([0u8; 8]);
            }
            return Err(e);
        }
        let mut file = file.unwrap();
        let mut xor: Vec<u8> = vec![];
        file.read_to_end(&mut xor)?;
        if xor.len() != 8 {
            panic!("Invalid xor.dat length.");
        }
        Ok(xor.try_into().unwrap())
    }
    fn read_file(&mut self, index: u32) -> Result<u32, ()> {
        let path = format!("{}/blk{:05}.dat", self.blocks_dir, index);
        //println!("Reading: {}", path);
        let block_reader = BlkFileReader::new(&path, self.xor);
        if block_reader.is_err() {
            self.data.write().unwrap().all_read = true;
            return Err(());
        }
        let mut block_reader = block_reader.unwrap();
        let mut block_count = 0;
        loop {
            // Read magic bytes.
            let mut magic = [0u8; 4];
            if block_reader.read_exact(&mut magic).is_err() {
                return Ok(block_count);
            }
            //println!("Magic bytes: {}", hex::encode(magic));
            // Read block size.
            let mut size = [0u8; 4];
            if block_reader.read_exact(&mut size).is_err() {
                return Ok(block_count);
            }
            let size = u32::from_le_bytes(size);
            //println!("Block size: {}", size);
            // Read block.
            let mut block_vec = vec![0u8; size as usize];
            if block_reader.read_exact(&mut block_vec).is_err() {
                return Ok(block_count);
            }
            block_count += 1;
            // Compute block hash.
            let block_hash = block_to_block_hash(&block_vec);
            let block_height = self.data.read().unwrap().block_height_by_hash.get(&block_hash).cloned();
            if block_height.is_none() {
                //println!("Block height not found for hash: {}", hex::encode(block_hash));
                continue;
            }
            let block_height = block_height.unwrap();
            //println!("Block height: {}", block_height);
            // Save blcok.
            self.data.write().unwrap().blocks.insert(block_height, Bytes::from(block_vec));
        }
    }
    pub fn read_next_file(&mut self) -> Result<u32, ()> {
        let next_blk_index = {
            let mut data = self.data.write().unwrap();
            let next_blk_index = data.next_blk_index;
            data.next_blk_index += 1;
            next_blk_index
        };
        let block_count = self.read_file(next_blk_index);
        if block_count.is_err() {
            return Err(());
        }
        Ok(block_count.unwrap())
    }
    pub fn try_get_next_block(&mut self) -> Option<(u32, Bytes)> {
        let mut data = self.data.write().unwrap();
        let next_height = data.next_height;
        if let Some(block) = data.blocks.remove(&next_height) {
            let height = data.next_height;
            data.next_height += 1;
            return Some((height, block));
        }
        None
    }
    pub fn get_next_block(&mut self) -> Option<(u32, Bytes)> {
        if self.data.read().unwrap().next_height > self.end_height {
            return None;
        }
        loop {
            let data = self.try_get_next_block();
            if data.is_some() {
                return data;
            }
            if self.data.read().unwrap().all_read {
                return None;
            }
            if self.read_next_file().is_err() {
                return None;
            }
        }
    }
}

impl Iterator for BlkReader {
    type Item = (u32, Bytes);
    fn next(&mut self) -> Option<Self::Item> {
        self.get_next_block()
    }
}
