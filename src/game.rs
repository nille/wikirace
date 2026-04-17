use crate::wiki::{Article, WikiClient};
use anyhow::Result;
use std::collections::HashSet;
use std::time::Instant;

/// Fuzzy match: all pattern chars must appear in order in the target.
/// Score rewards consecutive matches and matches at word boundaries.
fn fuzzy_score(pattern: &[char], target: &str) -> Option<i32> {
    let target_chars: Vec<char> = target.chars().collect();
    let mut pi = 0;
    let mut score = 0i32;
    let mut prev_match = false;

    for (ti, &tc) in target_chars.iter().enumerate() {
        if pi < pattern.len() && tc == pattern[pi] {
            score += 1;
            if prev_match {
                score += 2; // consecutive bonus
            }
            if ti == 0 || !target_chars[ti - 1].is_alphanumeric() {
                score += 3; // word boundary bonus
            }
            pi += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if pi == pattern.len() { Some(score) } else { None }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    Loading(String),
    Playing,
    Won,
    GaveUp,
}

pub struct Game {
    pub wiki: WikiClient,
    pub screen: Screen,
    pub start_title: String,
    pub target_title: String,
    pub target_extract: String,
    pub target_links: HashSet<String>,
    pub current: Option<Article>,
    pub path: Vec<String>,
    pub steps: usize,
    pub started_at: Option<Instant>,
    pub elapsed_secs: f64,
    pub link_offset: usize,
    pub selected: usize,
    pub filter: String,
    pub error: Option<String>,
    pub show_hints: bool,
    pub hints_loaded: bool,
    pub optimal_path: Option<Vec<String>>,
    pub searching_optimal: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            wiki: WikiClient::new(),
            screen: Screen::Loading("Picking random articles...".into()),
            start_title: String::new(),
            target_title: String::new(),
            target_extract: String::new(),
            target_links: HashSet::new(),
            current: None,
            path: Vec::new(),
            steps: 0,
            started_at: None,
            elapsed_secs: 0.0,
            link_offset: 0,
            selected: 0,
            filter: String::new(),
            error: None,
            show_hints: false,
            hints_loaded: false,
            optimal_path: None,
            searching_optimal: false,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.screen = Screen::Loading("Picking start article...".into());
        self.start_title = self.wiki.random_article().await?;

        self.screen = Screen::Loading("Picking target article...".into());
        self.target_title = self.wiki.random_article().await?;

        self.screen = Screen::Loading(format!("Loading {}...", self.target_title));
        let target = self.wiki.fetch_article(&self.target_title).await?;
        self.target_title = target.title.clone();
        self.target_extract = target.extract;

        self.screen = Screen::Loading(format!("Loading {}...", self.start_title));
        let article = self.wiki.fetch_article(&self.start_title).await?;
        self.start_title = article.title.clone();
        self.path.push(article.title.clone());
        self.current = Some(article);
        self.started_at = Some(Instant::now());
        self.screen = Screen::Playing;
        Ok(())
    }

    pub fn filtered_links(&self) -> Vec<&String> {
        let Some(article) = &self.current else {
            return vec![];
        };
        let mut links: Vec<&String> = if self.filter.is_empty() {
            article.links.iter().collect()
        } else {
            let pattern: Vec<char> = self.filter.to_lowercase().chars().collect();
            let mut scored: Vec<(&String, i32)> = article
                .links
                .iter()
                .filter_map(|link| {
                    fuzzy_score(&pattern, &link.to_lowercase()).map(|s| (link, s))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            scored.into_iter().map(|(link, _)| link).collect()
        };
        // Float hint/target links to the top when hints are on
        if self.show_hints && self.hints_loaded {
            links.sort_by(|a, b| {
                let a_hint = self.is_hint_link(a);
                let b_hint = self.is_hint_link(b);
                b_hint.cmp(&a_hint)
            });
        }
        links
    }

    pub fn is_hint_link(&self, link: &str) -> bool {
        if !self.show_hints || !self.hints_loaded {
            return false;
        }
        // A link is "warm" if it's a page that links TO the target (backlink)
        self.target_links.contains(&link.to_lowercase())
            || link.to_lowercase() == self.target_title.to_lowercase()
    }

    /// Fetch backlinks for the target (pages that link TO it). Called once on first hint toggle.
    pub async fn load_hints(&mut self) {
        if self.hints_loaded {
            return;
        }
        self.screen = Screen::Loading("Loading hints (fetching backlinks)...".into());
        match self.wiki.fetch_backlinks(&self.target_title).await {
            Ok(backlinks) => self.target_links = backlinks,
            Err(_) => {} // silently fail, hints just won't highlight
        }
        self.hints_loaded = true;
        self.screen = Screen::Playing;
    }

    pub fn selected_link(&self) -> Option<String> {
        self.filtered_links().get(self.selected).map(|s| s.to_string())
    }

    pub async fn navigate(&mut self, title: &str) -> Result<()> {
        self.error = None;
        self.screen = Screen::Loading(format!("Loading {title}..."));
        match self.wiki.fetch_article(title).await {
            Ok(article) => {
                self.steps += 1;
                self.path.push(article.title.clone());
                if article.title.to_lowercase() == self.target_title.to_lowercase() {
                    self.freeze_time();
                    self.current = Some(article);
                    self.screen = Screen::Won;
                } else {
                    self.current = Some(article);
                    self.selected = 0;
                    self.link_offset = 0;
                    self.filter.clear();
                    self.screen = Screen::Playing;
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to load: {e}"));
                self.screen = Screen::Playing;
            }
        }
        Ok(())
    }

    pub fn give_up(&mut self) {
        self.freeze_time();
        self.screen = Screen::GaveUp;
    }

    pub async fn find_optimal(&mut self) {
        self.searching_optimal = true;
        // BFS with max depth 4 to keep it reasonable
        match self
            .wiki
            .find_shortest_path(&self.start_title, &self.target_title, 4)
            .await
        {
            Ok(path) => self.optimal_path = path,
            Err(_) => self.optimal_path = None,
        }
        self.searching_optimal = false;
    }

    fn freeze_time(&mut self) {
        if let Some(start) = self.started_at {
            self.elapsed_secs = start.elapsed().as_secs_f64();
        }
    }

    pub fn tick(&mut self) {
        if let Some(start) = self.started_at {
            if self.screen == Screen::Playing {
                self.elapsed_secs = start.elapsed().as_secs_f64();
            }
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.filtered_links().len();
        if len == 0 {
            return;
        }
        let new = (self.selected as i32 + delta).rem_euclid(len as i32) as usize;
        self.selected = new;
    }

    pub fn format_time(&self) -> String {
        let secs = self.elapsed_secs as u64;
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}
