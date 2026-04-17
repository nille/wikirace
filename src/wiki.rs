use anyhow::Result;
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};

const API_URL: &str = "https://en.wikipedia.org/w/api.php";

#[derive(Clone, Debug)]
pub struct Article {
    pub title: String,
    pub extract: String,
    pub links: Vec<String>,
}

#[derive(Deserialize)]
struct QueryResponse {
    query: Option<QueryData>,
}

#[derive(Deserialize)]
struct QueryData {
    pages: HashMap<String, PageData>,
}

#[derive(Deserialize)]
struct PageData {
    title: Option<String>,
    extract: Option<String>,
    links: Option<Vec<LinkData>>,
}

#[derive(Deserialize)]
struct LinkData {
    title: String,
}

#[derive(Deserialize)]
struct RandomResponse {
    query: Option<RandomQuery>,
}

#[derive(Deserialize)]
struct RandomQuery {
    random: Vec<RandomPage>,
}

#[derive(Deserialize)]
struct RandomPage {
    title: String,
}

pub struct WikiClient {
    client: reqwest::Client,
}

impl WikiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("wikirace-tui/0.1 (https://github.com/wikirace)")
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_article(&self, title: &str) -> Result<Article> {
        let resp: QueryResponse = self
            .client
            .get(API_URL)
            .query(&[
                ("action", "query"),
                ("titles", title),
                ("prop", "extracts|links"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("pllimit", "max"),
                ("plnamespace", "0"),
                ("format", "json"),
                ("redirects", "1"),
            ])
            .send()
            .await?
            .json()
            .await?;

        let page = resp
            .query
            .as_ref()
            .and_then(|q| q.pages.values().next())
            .ok_or_else(|| anyhow::anyhow!("Article not found: {title}"))?;

        let mut links: Vec<String> = page
            .links
            .as_ref()
            .map(|l| l.iter().map(|link| link.title.clone()).collect())
            .unwrap_or_default();
        links.sort();

        Ok(Article {
            title: page.title.clone().unwrap_or_default(),
            extract: page.extract.clone().unwrap_or_default(),
            links,
        })
    }

    /// Fetch just the link titles for a page (no extract, lighter call)
    pub async fn fetch_links(&self, title: &str) -> Result<Vec<String>> {
        let resp: QueryResponse = self
            .client
            .get(API_URL)
            .query(&[
                ("action", "query"),
                ("titles", title),
                ("prop", "links"),
                ("pllimit", "max"),
                ("plnamespace", "0"),
                ("format", "json"),
                ("redirects", "1"),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(resp
            .query
            .as_ref()
            .and_then(|q| q.pages.values().next())
            .and_then(|p| p.links.as_ref())
            .map(|l| l.iter().map(|link| link.title.clone()).collect())
            .unwrap_or_default())
    }

    /// BFS shortest path from start to target, max depth to avoid runaway
    pub async fn find_shortest_path(
        &self,
        start: &str,
        target: &str,
        max_depth: usize,
    ) -> Result<Option<Vec<String>>> {
        let target_lower = target.to_lowercase();
        if start.to_lowercase() == target_lower {
            return Ok(Some(vec![start.to_string()]));
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<Vec<String>> = VecDeque::new();

        visited.insert(start.to_lowercase());
        queue.push_back(vec![start.to_string()]);

        while let Some(path) = queue.pop_front() {
            if path.len() > max_depth {
                break;
            }

            let current = path.last().unwrap();
            let links = match self.fetch_links(current).await {
                Ok(l) => l,
                Err(_) => continue,
            };

            for link in links {
                if link.to_lowercase() == target_lower {
                    let mut full_path = path.clone();
                    full_path.push(link);
                    return Ok(Some(full_path));
                }

                let key = link.to_lowercase();
                if !visited.contains(&key) {
                    visited.insert(key);
                    let mut new_path = path.clone();
                    new_path.push(link);
                    queue.push_back(new_path);
                }
            }
        }

        Ok(None)
    }

    /// Fetch pages that link TO the given title (backlinks)
    pub async fn fetch_backlinks(&self, title: &str) -> Result<HashSet<String>> {
        let resp: serde_json::Value = self
            .client
            .get(API_URL)
            .query(&[
                ("action", "query"),
                ("list", "backlinks"),
                ("bltitle", title),
                ("bllimit", "500"),
                ("blnamespace", "0"),
                ("format", "json"),
            ])
            .send()
            .await?
            .json()
            .await?;

        let mut set = HashSet::new();
        if let Some(backlinks) = resp
            .get("query")
            .and_then(|q| q.get("backlinks"))
            .and_then(|b| b.as_array())
        {
            for bl in backlinks {
                if let Some(title) = bl.get("title").and_then(|t| t.as_str()) {
                    set.insert(title.to_lowercase());
                }
            }
        }
        Ok(set)
    }

    pub async fn random_article(&self) -> Result<String> {
        let resp: RandomResponse = self
            .client
            .get(API_URL)
            .query(&[
                ("action", "query"),
                ("list", "random"),
                ("rnnamespace", "0"),
                ("rnlimit", "1"),
                ("format", "json"),
            ])
            .send()
            .await?
            .json()
            .await?;

        resp.query
            .and_then(|q| q.random.into_iter().next())
            .map(|p| p.title)
            .ok_or_else(|| anyhow::anyhow!("Failed to get random article"))
    }
}
