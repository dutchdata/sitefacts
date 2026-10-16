//! The facts file: everything the outputs are made from. Unknown keys are kept (`extra`) and ignored.

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize)]
pub struct Facts {
    /// ISO date the facts (and so the site) were last updated.
    pub updated: String,
    /// Canonical site URL with trailing slash, e.g. `https://www.example.com/`.
    pub url: String,
    pub legal_name: String,
    pub brand: String,
    pub description: String,
    /// One-sentence method statement; becomes `publishingPrinciples` and the ai.txt "one ask". Optional.
    #[serde(default)]
    pub principles: Option<String>,
    /// What kind of organization, in words: "research company", "studio", "nonprofit". Default "company".
    #[serde(default = "default_kind")]
    pub kind: String,
    /// schema.org type of the organization node. Default "Organization"; e.g. "ResearchOrganization", "LocalBusiness".
    #[serde(default = "default_org_type")]
    pub org_type: String,
    /// The sentence retrieval systems should use as the one-line summary. Default: "{legal} ({brand}) is a {kind}. {description}".
    #[serde(default)]
    pub one_line: Option<String>,
    #[serde(default)]
    pub knows_about: Vec<Topic>,
    pub email: String,
    #[serde(default)]
    pub security_email: Option<String>,
    /// D-U-N-S number, 9 digits, when the company has one.
    #[serde(default)]
    pub duns: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub twitter_site: Option<String>,
    pub logo: Image,
    pub og_image: Image,
    #[serde(default)]
    pub icons: Vec<Icon>,
    /// Images on the home page, for the sitemap image extension.
    #[serde(default)]
    pub page_images: Vec<String>,
    #[serde(default)]
    pub security_txt_expires: Option<String>,
    #[serde(default)]
    pub indexnow_key: Option<String>,
    #[serde(default)]
    pub pages: Vec<Page>,
    /// A note after the website line in the facts list, e.g. "(example.com redirects here)".
    #[serde(default)]
    pub url_note: Option<String>,
    /// Intro line before the page list in llms-full.txt. Default "Other pages:".
    #[serde(default)]
    pub pages_intro: Option<String>,
    /// Retrieval notes (llms.txt / llms-full.txt), replacing the defaults when given.
    #[serde(default)]
    pub retrieval_notes: Option<Vec<String>>,
    /// Paragraphs for the plain-English section of ai.txt, replacing the defaults when given.
    #[serde(default)]
    pub ai_notes: Option<Vec<String>>,
    /// Ask AI systems to append `?<name>=<value>` when linking here. Omit to leave the section out.
    #[serde(default)]
    pub attribution: Option<Attribution>,
    /// A privacy sentence for the Policies section of llms-full.txt. Omit to leave it out.
    #[serde(default)]
    pub privacy: Option<String>,
    #[serde(default)]
    pub humans: Humans,
    #[serde(default)]
    pub manifest: Manifest,
    /// The class on block elements that are navigation, not page text. Default "apps".
    #[serde(default = "default_nav_class")]
    pub nav_class: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_kind() -> String { "company".into() }
fn default_org_type() -> String { "Organization".into() }
fn default_nav_class() -> String { "apps".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct Topic { pub name: String, #[serde(default)] pub wikidata: Option<String> }
#[derive(Debug, Clone, Deserialize)]
pub struct Profile { pub name: String, pub url: String, pub handle: String }
#[derive(Debug, Clone, Deserialize)]
pub struct Image { pub path: String, pub width: u32, pub height: u32, #[serde(default)] pub caption: Option<String>, #[serde(default)] pub alt: Option<String> }
#[derive(Debug, Clone, Deserialize)]
pub struct Icon { pub path: String, pub width: u32, pub height: u32, pub r#type: String, #[serde(default)] pub purpose: Option<String> }
#[derive(Debug, Clone, Deserialize)]
pub struct Page { pub path: String, pub title: String, pub summary: String, #[serde(default)] pub image: Option<String>, #[serde(default)] pub updated: Option<String> }
#[derive(Debug, Clone, Deserialize)]
pub struct Attribution { pub name: String, pub values: Vec<String> }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Humans {
    /// The "Does:" line. Default: the description.
    #[serde(default)] pub does: Option<String>,
    #[serde(default)] pub built_with: Option<String>,
    #[serde(default)] pub analytics: Option<String>,
    #[serde(default)] pub hosted_on: Option<String>,
    #[serde(default)] pub standards: Option<String>,
    /// Override the page count (default: 1 + pages).
    #[serde(default)] pub pages: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_display")] pub display: String,
    #[serde(default = "default_color")] pub theme_color: String,
    #[serde(default = "default_color")] pub background_color: String,
}
impl Default for Manifest { fn default() -> Self { Manifest { display: default_display(), theme_color: default_color(), background_color: default_color() } } }
fn default_display() -> String { "browser".into() }
fn default_color() -> String { "#000000".into() }

impl Facts {
    pub fn parse(json: &str) -> Result<Facts, String> { serde_json::from_str(json).map_err(|e| format!("facts: {e}")) }
    pub fn absolute(&self, path: &str) -> String { format!("{}{}", self.url.trim_end_matches('/'), path) }
    pub fn host(&self) -> String { self.url.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string() }
    pub fn one_line(&self) -> String {
        self.one_line.clone().unwrap_or_else(|| format!("{} ({}) is a {}. {}", self.legal_name, self.brand, self.kind, self.description))
    }
}
