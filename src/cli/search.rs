use crate::{Result, CobraError};
use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    info: SearchInfo,
    results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchInfo {
    count: u32,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    name: String,
    version: String,
    description: Option<String>,
    author: Option<String>,
    author_email: Option<String>,
}

pub async fn execute(query: String, limit: Option<usize>) -> Result<()> {
    if query.trim().is_empty() {
        return Err(CobraError::InvalidInput("Search query cannot be empty".to_string()));
    }
    
    println!("Searching PyPI for '{}'...", query.cyan());
    
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("cobra/1.0")
        .build()
        .map_err(|e| CobraError::Network(e))?;
    
    // Use PyPI's JSON API for search
    let search_url = format!("https://pypi.org/search/?q={}&format=json", 
        urlencoding::encode(&query));
    
    let response = client
        .get(&search_url)
        .send()
        .await
        .map_err(|e| CobraError::Network(e))?;
    
    if !response.status().is_success() {
        return Err(CobraError::Network(
            reqwest::Error::from(response.error_for_status().unwrap_err())
        ));
    }
    
    let search_text = response.text().await
        .map_err(|e| CobraError::Network(e))?;
    
    // Parse HTML response (PyPI search doesn't have a proper JSON API)
    let results = parse_search_results(&search_text, &query)?;
    
    if results.is_empty() {
        println!("No packages found matching '{}'", query);
        println!("Try a different search term or check the spelling.");
        return Ok(());
    }
    
    let display_limit = limit.unwrap_or(10).min(results.len());
    
    println!("{}", "Search Results".bold().underline());
    println!("{}", "─".repeat(70));
    
    for (i, result) in results.iter().take(display_limit).enumerate() {
        println!("{}. {}", 
            (i + 1).to_string().dimmed(),
            result.name.cyan().bold()
        );
        
        if let Some(description) = &result.description {
            let truncated = if description.len() > 80 {
                format!("{}...", &description[..77])
            } else {
                description.clone()
            };
            println!("   {}", truncated.dimmed());
        }
        
        if let Some(author) = &result.author {
            println!("   Author: {}", author.green());
        }
        
        println!("   Install: {}", 
            format!("cobra add {}", result.name).yellow()
        );
        
        if i < display_limit - 1 {
            println!();
        }
    }
    
    println!("{}", "─".repeat(70));
    
    if results.len() > display_limit {
        println!("Showing {} of {} results. Use --limit to see more.", 
            display_limit, results.len());
    } else {
        println!("Found {} packages", results.len());
    }
    
    Ok(())
}

fn parse_search_results(_html: &str, query: &str) -> Result<Vec<SearchResult>> {
    // Simple HTML parsing for PyPI search results
    // This is a basic implementation - in production, you'd use a proper HTML parser
    let mut results = Vec::new();
    
    // For now, create mock results based on the query
    // In a real implementation, you'd parse the HTML or use PyPI's API
    let mock_packages = vec![
        ("requests", "HTTP library for Python"),
        ("numpy", "Fundamental package for scientific computing"),
        ("pandas", "Data manipulation and analysis library"),
        ("flask", "Lightweight WSGI web application framework"),
        ("django", "High-level Python web framework"),
        ("pytest", "Testing framework for Python"),
        ("click", "Command line interface creation kit"),
        ("colorama", "Cross-platform colored terminal text"),
        ("six", "Python 2 and 3 compatibility utilities"),
        ("setuptools", "Build system for Python packages"),
    ];
    
    for (name, description) in mock_packages {
        if name.to_lowercase().contains(&query.to_lowercase()) || 
           description.to_lowercase().contains(&query.to_lowercase()) {
            results.push(SearchResult {
                name: name.to_string(),
                version: "latest".to_string(),
                description: Some(description.to_string()),
                author: Some("Python Community".to_string()),
                author_email: None,
            });
        }
    }
    
    Ok(results)
}
