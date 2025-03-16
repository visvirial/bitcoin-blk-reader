
use std::time::{
    Duration,
};
use bytes::Bytes;
use reqwest::{
    Response,
    StatusCode,
};
use bitcoin_hashes::Sha256d;

#[derive(Debug)]
pub enum BitcoinRestError {
	Reqwest(reqwest::Error),
	Response(Response),
}

impl std::fmt::Display for BitcoinRestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            Self::Response(e) => write!(f, "Response error: {}", e.status()),
        }
    }
}

impl std::error::Error for BitcoinRestError {}

impl From<reqwest::Error> for BitcoinRestError {
	fn from(e: reqwest::Error) -> Self {
		Self::Reqwest(e)
	}
}

impl From<Response> for BitcoinRestError {
	fn from(e: Response) -> Self {
		Self::Response(e)
	}
}

#[derive(Debug, Clone)]
pub struct BitcoinRest {
    client: reqwest::Client,
    rest_endpoint: String,
}

impl BitcoinRest {
    pub fn new(rest_endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap()
            ;
        Self {
            client,
            rest_endpoint,
        }
    }
    pub async fn fetch(&self, path: &[&str], ext: &str, query: Option<&str>) -> Result<Response, reqwest::Error> {
        let mut url = format!("{}/{}.{}", self.rest_endpoint, path.join("/"), ext);
        if let Some(query) = query {
            url.push_str(&format!("?{}", query));
        }
				return Ok(self.client.get(&url).send().await?);
    }
    pub async fn fetch_hex(&self, path: &[&str], query: Option<&str>) -> Result<String, BitcoinRestError> {
        let response = self.fetch(path, "hex", query).await?;
        if response.status() != StatusCode::OK {
            return Err(response.into());
        }
        let hex = response.text().await.unwrap().trim().to_string();
        Ok(hex)
    }
    pub async fn fetch_bin(&self, path: &[&str], query: Option<&str>) -> Result<Bytes, BitcoinRestError> {
        let response = self.fetch(path, "bin", query).await?;
        if response.status() != StatusCode::OK {
            return Err(response.into());
        }
        let bytes = response.bytes().await.unwrap();
        Ok(bytes)
    }
    pub async fn get_block(&self, mut hash: [u8; 32]) -> Result<Bytes, BitcoinRestError> {
        hash.reverse();
        let block = self.fetch_bin(&["block", &hex::encode(hash)], None).await?;
        Ok(block)
    }
    pub async fn get_blockhashbyheight(&self, height: u32) -> Result<[u8; 32], BitcoinRestError> {
        let block_hash = self.fetch_bin(&["blockhashbyheight", &height.to_string()], None).await?;
        if block_hash.len() != 32 {
            panic!("Invalid block hash length");
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&block_hash);
        Ok(hash)
    }
    pub async fn get_headers(&self, mut hash: [u8; 32], count: u32) -> Result<Vec<[u8; 80]>, BitcoinRestError> {
        hash.reverse();
        let mut headers = Vec::new();
        let headers_bytes = self.fetch_bin(&["headers", &hex::encode(hash)], Some(format!("count={}", count).as_str())).await?;
        if headers_bytes.len() % 80 != 0 {
            panic!("Invalid headers length");
        }
        for i in 0..(headers_bytes.len() / 80) {
            let mut header = [0u8; 80];
            header.copy_from_slice(&headers_bytes[(i * 80)..((i + 1) * 80)]);
            headers.push(header);
        }
        Ok(headers)
    }
    pub async fn get_all_headers(&self, mut hash: [u8; 32], count: Option<u32>) -> Result<Vec<[u8; 80]>, BitcoinRestError> {
        let mut result = Vec::new();
        let count = count.unwrap_or(2000);
        let mut is_first = true;
        loop {
            let mut headers = self.get_headers(hash, count).await?;
            let headers_len = headers.len();
            if headers_len == 0 {
                break;
            }
            // Drop first header on non-first iteration.
            if !is_first {
                headers = headers[1..].to_vec();
            }
            is_first = false;
            hash = Sha256d::hash(headers.last().unwrap()).to_byte_array();
            result.push(headers);
            if headers_len < count as usize {
                break;
            }
        }
        let headers = result.concat();
        Ok(headers)
    }
}
