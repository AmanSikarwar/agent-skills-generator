//! Crawler module for the agent-skills-generator.
//!
//! This module provides the web crawling functionality using the `spider` crate.
//! It handles:
//! - Website initialization with proper configuration
//! - Asynchronous page subscription and processing
//! - Respect for robots.txt and polite crawling delays
//! - URL filtering based on configuration rules using globset

use crate::config::Config;
use crate::processor::Processor;
use anyhow::{Context, Result};
use spider::page::Page;
use spider::website::Website;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

/// Statistics for a crawl session.
#[derive(Debug, Default)]
pub struct CrawlStats {
    /// Total pages visited.
    pub pages_visited: AtomicUsize,
    /// Pages successfully processed.
    pub pages_processed: AtomicUsize,
    /// Pages skipped due to rules.
    pub pages_skipped: AtomicUsize,
    /// Pages that failed to process.
    pub pages_failed: AtomicUsize,
}

impl CrawlStats {
    /// Creates a new stats tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a summary of the crawl.
    pub fn summary(&self) -> String {
        format!(
            "Crawl complete: {} visited, {} processed, {} skipped, {} failed",
            self.pages_visited.load(Ordering::Relaxed),
            self.pages_processed.load(Ordering::Relaxed),
            self.pages_skipped.load(Ordering::Relaxed),
            self.pages_failed.load(Ordering::Relaxed),
        )
    }
}

/// Web crawler that processes pages and generates skill files.
pub struct Crawler {
    /// Configuration for the crawler.
    config: Config,
    /// Content processor - stored for potential future use in custom processing.
    #[allow(dead_code)]
    processor: Processor,
    /// Output directory for generated skills.
    output_dir: PathBuf,
    /// Crawl statistics.
    stats: Arc<CrawlStats>,
}

impl Crawler {
    /// Creates a new crawler with the given configuration.
    pub fn new(config: Config, output_dir: PathBuf) -> Result<Self> {
        let processor = Processor::new(&config)?;

        // Validate that URL filter can be built from config
        config.build_url_filter()?;

        Ok(Self {
            config,
            processor,
            output_dir,
            stats: Arc::new(CrawlStats::new()),
        })
    }

    /// Returns the current crawl statistics.
    pub fn stats(&self) -> &Arc<CrawlStats> {
        &self.stats
    }

    /// Crawls a website and generates skill files.
    ///
    /// # Arguments
    /// * `url` - The starting URL to crawl
    ///
    /// # Returns
    /// The crawl statistics on success.
    pub async fn crawl(&self, url: &str) -> Result<Arc<CrawlStats>> {
        info!("Starting crawl of: {}", url);

        // Ensure output directory exists
        fs_err::tokio::create_dir_all(&self.output_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create output directory: {}",
                    self.output_dir.display()
                )
            })?;

        // Initialize the website with configuration
        let mut website = Website::new(url);

        // Configure the website
        self.configure_website(&mut website);

        // Subscribe to page events with a buffer
        let mut rx = website
            .subscribe(self.config.concurrency * 2)
            .context("Failed to subscribe to page events")?;

        // Semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Clone references for the spawned task
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        let output_dir = self.output_dir.clone();
        let processor = Processor::new(&config)?;

        // Build URL filter for the spawned task
        let url_filter = config.build_url_filter()?;

        debug!(
            "URL filter built with {} rules (has_allow_rules: {})",
            config.rules.len(),
            config.has_allow_rules()
        );

        // Spawn a task to process pages as they come in
        let process_handle = tokio::spawn(async move {
            while let Ok(page) = rx.recv().await {
                let url = page.get_url().to_string();

                stats.pages_visited.fetch_add(1, Ordering::Relaxed);

                // Check if URL should be crawled based on rules using UrlFilter
                if !url_filter.should_crawl(&url) {
                    debug!("Skipping URL due to rules: {}", url);
                    stats.pages_skipped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                // Acquire semaphore permit for concurrency control
                let permit = semaphore.clone().acquire_owned().await;
                if permit.is_err() {
                    warn!("Failed to acquire semaphore permit");
                    continue;
                }
                let _permit = permit.unwrap();

                // Process the page
                match Self::process_page(&processor, &page, &output_dir).await {
                    Ok(skill_dir) => {
                        info!("Processed: {} -> {}", url, skill_dir.display());
                        stats.pages_processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        error!("Failed to process {}: {:?}", url, e);
                        stats.pages_failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        // Start the crawl
        website.crawl().await;

        // Unsubscribe to close the channel and signal completion
        website.unsubscribe();

        // Wait for processing to complete
        // The receiver will complete when the channel is closed
        let _ = process_handle.await;

        info!("{}", self.stats.summary());

        Ok(Arc::clone(&self.stats))
    }

    /// Configures the spider Website with our settings.
    fn configure_website(&self, website: &mut Website) {
        // Set user agent
        if let Some(ref user_agent) = self.config.user_agent {
            website.with_user_agent(Some(user_agent.as_str()));
        } else {
            website.with_user_agent(Some(
                "AgentSkillsGenerator/1.0 (+https://github.com/agentskills/generator)",
            ));
        }

        // Configure politeness settings
        website.configuration.delay = self.config.delay_ms;
        website.configuration.respect_robots_txt = self.config.respect_robots_txt;
        website.configuration.subdomains = self.config.subdomains;
        website.configuration.depth = self.config.max_depth;

        // Set request timeout
        website.configuration.request_timeout = Some(Box::new(Duration::from_secs(
            self.config.request_timeout_secs,
        )));

        // Configure whitelist from allow rules - these are regex patterns
        // Spider will ONLY crawl URLs matching these patterns
        let whitelist = self.config.get_whitelist_regex_patterns();
        if !whitelist.is_empty() {
            info!(
                "Configuring whitelist with {} patterns - spider will only visit matching URLs",
                whitelist.len()
            );
            for pattern in &whitelist {
                info!("Whitelist regex: {}", pattern);
            }
            let whitelist_vec: Vec<spider::compact_str::CompactString> =
                whitelist.into_iter().map(|s| s.into()).collect();
            // Use the proper method to set whitelist and configure it
            website.with_whitelist_url(Some(whitelist_vec));
        }

        // Configure blacklist from ignore rules - these are checked even when whitelist exists
        // This allows user-defined ignore patterns to exclude specific paths
        let blacklist = self.config.get_blacklist_patterns();
        if !blacklist.is_empty() {
            info!("Configuring blacklist with {} patterns", blacklist.len());
            for pattern in &blacklist {
                info!("Blacklist regex: {}", pattern);
            }
            let blacklist_vec: Vec<spider::compact_str::CompactString> =
                blacklist.into_iter().map(|s| s.into()).collect();
            website.with_blacklist_url(Some(blacklist_vec));
        }

        // Compile the allowlist/blocklist if any patterns were configured
        if !self.config.get_whitelist_regex_patterns().is_empty()
            || !self.config.get_blacklist_patterns().is_empty()
        {
            website.configuration.configure_allowlist();
        }

        // Only crawl HTML pages
        website.configuration.only_html = true;

        debug!("Website configured: {:?}", website.configuration);
    }

    /// Processes a single page.
    async fn process_page(
        processor: &Processor,
        page: &Page,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let url = page.get_url();
        let html = page.get_html();

        if html.is_empty() {
            anyhow::bail!("Empty HTML content for: {}", url);
        }

        // Process the page
        let processed = processor
            .process(url, &html)
            .with_context(|| format!("Failed to process page: {}", url))?;

        // Write to disk
        let skill_dir = processor
            .write_to_disk(&processed, output_dir)
            .await
            .with_context(|| format!("Failed to write skill for: {}", url))?;

        Ok(skill_dir)
    }
}

/// Cleans up the output directory by removing all generated skills.
pub async fn clean_output_dir(output_dir: &PathBuf) -> Result<usize> {
    use fs_err::tokio as fs;

    if !output_dir.exists() {
        info!("Output directory does not exist: {}", output_dir.display());
        return Ok(0);
    }

    let mut count = 0;
    let mut entries = fs::read_dir(output_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            // Check if it looks like a skill directory (has SKILL.md)
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                fs::remove_dir_all(&path).await.with_context(|| {
                    format!("Failed to remove skill directory: {}", path.display())
                })?;
                count += 1;
                debug!("Removed: {}", path.display());
            }
        }
    }

    info!("Cleaned {} skill directories", count);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_stats() {
        let stats = CrawlStats::new();

        stats.pages_visited.fetch_add(10, Ordering::Relaxed);
        stats.pages_processed.fetch_add(8, Ordering::Relaxed);
        stats.pages_skipped.fetch_add(1, Ordering::Relaxed);
        stats.pages_failed.fetch_add(1, Ordering::Relaxed);

        let summary = stats.summary();
        assert!(summary.contains("10 visited"));
        assert!(summary.contains("8 processed"));
        assert!(summary.contains("1 skipped"));
        assert!(summary.contains("1 failed"));
    }

    #[tokio::test]
    async fn test_crawler_creation() {
        let config = Config::default();
        let output_dir = PathBuf::from("/tmp/test-skills");

        let crawler = Crawler::new(config, output_dir);
        assert!(crawler.is_ok());
    }
}
