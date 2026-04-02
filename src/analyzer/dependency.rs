//! Dependency analysis using cargo_metadata

use crate::error::Result;
use cargo_metadata::{DependencyKind as CargoDependencyKind, MetadataCommand, Package};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::Path;

/// Analyzer for crate dependencies using cargo_metadata
pub struct DependencyAnalyzer {
    metadata: cargo_metadata::Metadata,
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

/// Information about a crate
#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub dependencies: Vec<DependencyInfo>,
    pub features: Vec<String>,
    pub default_features: Vec<String>,
    pub edition: String,
    pub rust_version: Option<String>,
}

/// Information about a dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub optional: bool,
    pub features: Vec<String>,
    pub kind: DependencyKind,
}

/// Kind of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

impl std::fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyKind::Normal => write!(f, "normal"),
            DependencyKind::Dev => write!(f, "dev"),
            DependencyKind::Build => write!(f, "build"),
        }
    }
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer from a Cargo.toml path
    pub fn from_manifest(manifest_path: &Path) -> Result<Self> {
        let metadata = MetadataCommand::new().manifest_path(manifest_path).exec()?;
        Ok(Self::from_metadata(metadata))
    }

    /// Create a new dependency analyzer from the current directory
    pub fn from_current_dir() -> Result<Self> {
        let metadata = MetadataCommand::new().exec()?;
        Ok(Self::from_metadata(metadata))
    }

    fn from_metadata(metadata: cargo_metadata::Metadata) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Build dependency graph
        for package in &metadata.packages {
            let node = graph.add_node(package.name.clone());
            node_map.insert(package.name.clone(), node);
        }

        for package in &metadata.packages {
            if let Some(&from_node) = node_map.get(&package.name) {
                for dep in &package.dependencies {
                    if let Some(&to_node) = node_map.get(&dep.name) {
                        graph.add_edge(from_node, to_node, ());
                    }
                }
            }
        }

        Self {
            metadata,
            graph,
            node_map,
        }
    }

    /// Get the root package (if this is a single-crate project)
    pub fn root_package(&self) -> Option<CrateInfo> {
        self.metadata
            .root_package()
            .map(|pkg| self.package_to_info(pkg))
    }

    /// Get information about a specific crate
    pub fn get_crate_info(&self, name: &str) -> Option<CrateInfo> {
        self.metadata
            .packages
            .iter()
            .find(|p| p.name == name)
            .map(|pkg| self.package_to_info(pkg))
    }

    /// Get all packages in the workspace
    pub fn all_packages(&self) -> Vec<CrateInfo> {
        self.metadata
            .packages
            .iter()
            .map(|pkg| self.package_to_info(pkg))
            .collect()
    }

    /// Get direct dependencies of a crate
    pub fn direct_dependencies(&self, name: &str) -> Vec<DependencyInfo> {
        self.metadata
            .packages
            .iter()
            .find(|p| p.name == name)
            .map(|pkg| self.extract_dependencies(pkg))
            .unwrap_or_default()
    }

    /// Get the dependency tree as a flat list with depth indicators
    pub fn dependency_tree(&self, root: &str) -> Vec<(String, usize)> {
        if let Some(&root_node) = self.node_map.get(root) {
            let mut result = Vec::new();
            let mut visited = HashMap::new();
            self.traverse_deps(root_node, 0, &mut result, &mut visited);
            result
        } else {
            Vec::new()
        }
    }

    /// Get total number of dependencies (transitive)
    pub fn total_dependency_count(&self, name: &str) -> usize {
        self.dependency_tree(name).len().saturating_sub(1)
    }

    fn traverse_deps(
        &self,
        node: NodeIndex,
        depth: usize,
        result: &mut Vec<(String, usize)>,
        visited: &mut HashMap<NodeIndex, bool>,
    ) {
        if visited.contains_key(&node) {
            return;
        }

        visited.insert(node, true);
        let name = self.graph[node].clone();
        result.push((name, depth));

        for neighbor in self.graph.neighbors(node) {
            self.traverse_deps(neighbor, depth + 1, result, visited);
        }
    }

    fn package_to_info(&self, pkg: &Package) -> CrateInfo {
        let dependencies = self.extract_dependencies(pkg);
        let features: Vec<String> = pkg.features.keys().cloned().collect();
        let default_features = pkg.features.get("default").cloned().unwrap_or_default();

        CrateInfo {
            name: pkg.name.clone(),
            version: pkg.version.to_string(),
            authors: pkg.authors.clone(),
            license: pkg.license.clone(),
            description: pkg.description.clone(),
            homepage: pkg.homepage.clone(),
            repository: pkg.repository.clone(),
            documentation: pkg.documentation.clone(),
            dependencies,
            features,
            default_features,
            edition: pkg.edition.to_string(),
            rust_version: pkg.rust_version.as_ref().map(|v| v.to_string()),
        }
    }

    fn extract_dependencies(&self, pkg: &Package) -> Vec<DependencyInfo> {
        pkg.dependencies
            .iter()
            .map(|dep| DependencyInfo {
                name: dep.name.clone(),
                version: dep.req.to_string(),
                optional: dep.optional,
                features: dep.features.clone(),
                kind: match dep.kind {
                    CargoDependencyKind::Normal => DependencyKind::Normal,
                    CargoDependencyKind::Development => DependencyKind::Dev,
                    CargoDependencyKind::Build => DependencyKind::Build,
                    _ => DependencyKind::Normal,
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_dependency_tree_from_manifest() {
        // Run from crate root: Cargo.toml exists
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        if !manifest.exists() {
            return;
        }
        let analyzer = DependencyAnalyzer::from_manifest(&manifest).unwrap();
        let root = analyzer.root_package().expect("root package");
        assert_eq!(root.name, "vizier-tui");
        let tree = analyzer.dependency_tree(&root.name);
        assert!(!tree.is_empty());
        assert_eq!(tree[0].0, root.name);
        assert_eq!(tree[0].1, 0);
    }

    #[test]
    fn test_direct_dependencies() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        if !manifest.exists() {
            return;
        }
        let analyzer = DependencyAnalyzer::from_manifest(&manifest).unwrap();
        let root = analyzer.root_package().unwrap();
        let deps = analyzer.direct_dependencies(&root.name);
        // Vizier has at least ratatui, crossterm, etc.
        assert!(deps
            .iter()
            .any(|d| d.name == "ratatui" || d.name == "crossterm"));
    }
}
