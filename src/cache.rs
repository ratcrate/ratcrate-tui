use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use colored::*;

use crate::types::CratesData;

const REMOTE_URL: &str = "https://ratcrate.github.io/data/ratcrate.json";
const CACHE_MAX_AGE_DAYS: u64 = 7;

/// Get the cache directory path
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("ratcrate")
    } else {
        dirs::cache_dir()
            .context("Failed to get cache directory")?
            .join("ratcrate")
    };
    
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Get the cache file path
pub fn get_cache_file() -> Result<PathBuf> {
    Ok(get_cache_dir()?.join("ratcrate.json"))
}

/// Check if cache is stale
pub fn is_cache_stale() -> Result<bool> {
    let cache_file = get_cache_file()?;
    
    if !cache_file.exists() {
        return Ok(true);
    }
    
    let metadata = fs::metadata(&cache_file)?;
    let modified = metadata.modified()?;
    let age = SystemTime::now().duration_since(modified)?;
    
    Ok(age > Duration::from_secs(CACHE_MAX_AGE_DAYS * 24 * 3600))
}

/// Load data from cache
pub fn load_from_cache() -> Result<CratesData> {
    let cache_file = get_cache_file()?;
    let content = fs::read_to_string(&cache_file)
        .context("Failed to read cache file")?;
    
    let data: CratesData = serde_json::from_str(&content)
        .context("Failed to parse cache file")?;
    
    println!("{}", "✓ Loaded from cache".green());
    Ok(data)
}

/// Download fresh data from GitHub
pub fn download_fresh_data() -> Result<CratesData> {
    println!("{}", "📡 Downloading latest data from GitHub...".cyan());
    
    let response = reqwest::blocking::get(REMOTE_URL)
        .context("Failed to download data")?;
    
    if !response.status().is_success() {
        anyhow::bail!("Server returned status: {}", response.status());
    }
    
    let data: CratesData = response.json()
        .context("Failed to parse downloaded data")?;
    
    // Save to cache
    let cache_file = get_cache_file()?;
    let json = serde_json::to_string_pretty(&data)?;
    fs::write(&cache_file, json)?;
    
    println!(
        "{}",
        format!("✓ Downloaded and cached {} crates", data.metadata.total_crates).green()
    );
    
    Ok(data)
}

/// Get data - use cache if fresh, otherwise download
pub fn get_data(force_refresh: bool) -> Result<CratesData> {
    if force_refresh {
        println!("{}", "🔄 Force refresh requested".yellow());
        download_fresh_data()
    } else if is_cache_stale()? {
        println!("{}", "⚠ Cache is stale, downloading fresh data...".yellow());
        download_fresh_data()
    } else {
        load_from_cache()
    }
}
