use anyhow::{Context, Result};
use pulldown_cmark::{Event, Parser, Tag};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Requirement metadata from frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementMeta {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub owner: String,
}

/// Acceptance Criterion (scenario)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// Complete requirement with metadata and scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub meta: RequirementMeta,
    pub content: String,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub hash: String,
}

impl Requirement {
    /// Parse a markdown file with frontmatter
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .context(format!("Failed to read file: {:?}", path.as_ref()))?;
        Self::from_string(&content)
    }

    /// Parse markdown content
    pub fn from_string(content: &str) -> Result<Self> {
        // Split frontmatter and content
        let (frontmatter, body) = Self::split_frontmatter(content)?;
        
        // Parse frontmatter
        let meta: RequirementMeta = serde_yaml::from_str(&frontmatter)
            .context("Failed to parse frontmatter YAML")?;

        // Parse acceptance criteria from headings
        let acceptance_criteria = Self::parse_acceptance_criteria(&body, &meta.id);

        // Calculate hash of the effective content
        let hash = Self::calculate_hash(&body);

        Ok(Requirement {
            meta,
            content: body.trim().to_string(),
            acceptance_criteria,
            hash,
        })
    }

    /// Split frontmatter and content
    fn split_frontmatter(content: &str) -> Result<(String, String)> {
        let lines: Vec<&str> = content.lines().collect();
        
        // Check if starts with ---
        if !lines.first().map_or(false, |l| l.trim() == "---") {
            return Ok((String::new(), content.to_string()));
        }

        // Find closing ---
        let end_idx = lines[1..]
            .iter()
            .position(|l| l.trim() == "---")
            .context("Frontmatter not closed with ---")?;

        let frontmatter = lines[1..=end_idx].join("\n");
        let body = lines[end_idx + 2..].join("\n");

        Ok((frontmatter, body))
    }

    /// Parse acceptance criteria from markdown headings
    fn parse_acceptance_criteria(content: &str, req_id: &str) -> Vec<AcceptanceCriterion> {
        let parser = Parser::new(content);
        let mut criteria = Vec::new();
        let mut current_heading: Option<(usize, String)> = None;
        let mut ac_content = String::new();
        let mut ac_counter = 1;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) if level == pulldown_cmark::HeadingLevel::H3 => {
                    // Save previous AC if exists
                    if let Some((_, title)) = current_heading.take() {
                        if title.to_uppercase().contains("AC-") || title.contains("验收") {
                            let ac_id = format!("{}.AC-{:02}", req_id, ac_counter);
                            criteria.push(AcceptanceCriterion {
                                id: ac_id,
                                title: title.clone(),
                                content: ac_content.trim().to_string(),
                            });
                            ac_counter += 1;
                            ac_content.clear();
                        }
                    }
                    current_heading = Some((3, String::new()));
                }
                Event::Text(text) if current_heading.is_some() => {
                    if let Some((_, ref mut heading_text)) = current_heading {
                        heading_text.push_str(&text);
                    }
                }
                Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                    // Heading text is complete
                }
                Event::Text(text) | Event::Code(text) if current_heading.is_some() => {
                    ac_content.push_str(&text);
                }
                Event::SoftBreak | Event::HardBreak => {
                    ac_content.push('\n');
                }
                _ => {}
            }
        }

        // Don't forget the last AC
        if let Some((_, title)) = current_heading {
            if title.to_uppercase().contains("AC-") || title.contains("验收") {
                let ac_id = format!("{}.AC-{:02}", req_id, ac_counter);
                criteria.push(AcceptanceCriterion {
                    id: ac_id,
                    title,
                    content: ac_content.trim().to_string(),
                });
            }
        }

        criteria
    }

    /// Calculate SHA256 hash of content
    fn calculate_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        // Only hash non-empty lines to ignore formatting changes
        let effective_content: String = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        hasher.update(effective_content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get all traceable IDs (requirement ID + all AC IDs)
    pub fn get_all_ids(&self) -> Vec<String> {
        let mut ids = vec![self.meta.id.clone()];
        ids.extend(self.acceptance_criteria.iter().map(|ac| ac.id.clone()));
        ids
    }
}

// RequirementStore for future use when scanning multiple requirement files
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct RequirementStore {
    requirements: HashMap<String, Requirement>,
}

#[allow(dead_code)]
impl RequirementStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, req: Requirement) {
        self.requirements.insert(req.meta.id.clone(), req);
    }

    pub fn get(&self, id: &str) -> Option<&Requirement> {
        self.requirements.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &Requirement> {
        self.requirements.values()
    }

    pub fn len(&self) -> usize {
        self.requirements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }
}
