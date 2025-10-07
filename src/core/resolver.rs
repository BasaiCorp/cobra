use crate::{Result, CobraError, Package, Dependency, constants::*};
use crate::core::cache::MultiLevelCache;
use crate::registry::client::RegistryClient;
use petgraph::Graph;
use petgraph::algo::toposort;
use rayon::prelude::*;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use rustc_hash::FxHashMap;

pub struct DependencyResolver {
    client: Arc<RegistryClient>,
    cache: Option<Arc<MultiLevelCache>>,
}

impl DependencyResolver {
    pub fn new(client: Arc<RegistryClient>, cache: Option<Arc<MultiLevelCache>>) -> Self {
        Self { client, cache }
    }

    /// Resolve dependencies in parallel with topological sorting
    pub async fn resolve(&self, dependencies: &[Dependency]) -> Result<Vec<Package>> {
        if dependencies.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch metadata for all packages in parallel
        let futures: Vec<_> = dependencies.iter()
            .map(|dep| self.fetch_package_metadata(&dep.name, &dep.version_spec))
            .collect();

        let packages = futures::future::try_join_all(futures).await?;

        // Build dependency graph
        let mut graph = Graph::<String, ()>::new();
        let mut node_map: FxHashMap<String, _> = FxHashMap::default();
        let mut all_packages: FxHashMap<String, Package> = FxHashMap::default();

        // Add root packages
        for pkg in &packages {
            let node = graph.add_node(format!("{}@{}", pkg.name, pkg.version));
            node_map.insert(format!("{}@{}", pkg.name, pkg.version), node);
            all_packages.insert(format!("{}@{}", pkg.name, pkg.version), pkg.clone());
        }

        // Recursively resolve dependencies
        let mut to_process: Vec<Package> = packages.clone();
        let mut processed: HashSet<String> = HashSet::new();

        while let Some(pkg) = to_process.pop() {
            let pkg_key = format!("{}@{}", pkg.name, pkg.version);
            
            if processed.contains(&pkg_key) {
                continue;
            }
            processed.insert(pkg_key.clone());

            // Fetch dependencies in parallel
            if !pkg.dependencies.is_empty() {
                let dep_futures: Vec<_> = pkg.dependencies.iter()
                    .map(|dep| self.fetch_package_metadata(&dep.name, &dep.version_spec))
                    .collect();

                let dep_packages = futures::future::try_join_all(dep_futures).await?;

                for dep_pkg in dep_packages {
                    let dep_key = format!("{}@{}", dep_pkg.name, dep_pkg.version);
                    
                    // Add node if not exists
                    if !node_map.contains_key(&dep_key) {
                        let node = graph.add_node(dep_key.clone());
                        node_map.insert(dep_key.clone(), node);
                        all_packages.insert(dep_key.clone(), dep_pkg.clone());
                        to_process.push(dep_pkg);
                    }

                    // Add edge from package to dependency
                    if let (Some(&from), Some(&to)) = (node_map.get(&pkg_key), node_map.get(&dep_key)) {
                        graph.add_edge(from, to, ());
                    }
                }
            }
        }

        // Topological sort for install order
        let sorted = toposort(&graph, None)
            .map_err(|_| CobraError::ResolutionFailed("Circular dependency detected".to_string()))?;

        // Return packages in install order (reverse topological order)
        let mut result = Vec::new();
        for node in sorted.iter().rev() {
            let pkg_key = &graph[*node];
            if let Some(pkg) = all_packages.get(pkg_key) {
                result.push(pkg.clone());
            }
        }

        Ok(result)
    }

    async fn fetch_package_metadata(&self, name: &str, version_spec: &str) -> Result<Package> {
        // Check cache first
        if let Some(cache) = &self.cache {
            let cache_key = format!("metadata:{}:{}", name, version_spec);
            if let Some(data) = cache.get(&cache_key).await {
                if let Ok(pkg) = serde_json::from_slice::<Package>(&data) {
                    return Ok(pkg);
                }
            }
        }

        // Fetch from registry
        let pkg = self.client.get_package_info(name, version_spec).await?;

        // Cache the result
        if let Some(cache) = &self.cache {
            let cache_key = format!("metadata:{}:{}", name, version_spec);
            if let Ok(data) = serde_json::to_vec(&pkg) {
                let _ = cache.put(cache_key, bytes::Bytes::from(data)).await;
            }
        }

        Ok(pkg)
    }
}
