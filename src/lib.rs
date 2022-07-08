use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

lazy_static! {
    static ref FILENAME_RE: Regex = RegexBuilder::new(
        r"
          (\d{8}T\d{6})
          --
          (.*?)
          __
          (.*)
          \.
          ([a-z]+)
        "
    )
    .ignore_whitespace(true)
    .build()
    .expect("syntax error in static regex");
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error")]
    ParseError(String),
    #[error("io error")]
    IOError(String),
}

use Error::*;

pub type Result<T> = std::result::Result<T, Error>;

fn slugify(title: &str) -> String {
    title.to_ascii_lowercase().replace(' ', "-")
}

pub fn name_from_relative_path(relative_path: &Path) -> String {
    relative_path.to_string_lossy().replace('/', "")
}

pub fn parse_file_name(name: &str) -> Result<Metadata> {
    let captures = FILENAME_RE.captures(name).ok_or_else(|| {
        ParseError(format!("Filename {name} did not match expected regex").to_string())
    })?;

    let id = captures
        .get(1)
        .expect("FILENAME_RE should contain the correct number of groups")
        .as_str();
    let id = Id::from_str(id)?;

    let slug = captures
        .get(2)
        .expect("FILENAME_RE should contain the correct number of groups")
        .as_str()
        .to_owned();

    let keywords: Vec<String> = captures
        .get(3)
        .expect("FILENAME_RE should contain the correct number of groups")
        .as_str()
        .split('_')
        .map(|x| x.to_string())
        .collect();

    let extension = captures
        .get(4)
        .expect("FILENAME_RE should contain the correct number of groups")
        .as_str()
        .to_owned();

    Ok(Metadata {
        id,
        slug,
        title: None,
        keywords,
        extension,
    })
}

fn try_extract_front_matter(contents: &str) -> Option<(FrontMatter, String)> {
    let docs: Vec<_> = contents.splitn(3, "---\n").collect();
    if docs.is_empty() {
        println!("skipping empty front_matter");
        return None;
    }
    if docs.len() < 2 {
        println!("skipping invalid front_matter");
        return None;
    }
    let first_doc = &docs[1];
    let text = docs[2];
    match FrontMatter::parse(&first_doc) {
        Ok(f) => Some((f, text.to_string())),
        Err(ParseError(e)) => {
            println!("skipping invalid front_matter: {}", e);
            None
        }
        Err(_) => {
            unreachable!()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Id(String);

impl Id {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn human_date(&self) -> String {
        let ymd = &self.0[0..8];
        let hms = &self.0[9..];
        format!("{ymd} {hms}")
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let chars: Vec<char> = s.chars().collect();

        if chars.len() != 15 {
            return Err(ParseError(format!(
                "value '{s}' should contain 15 characters, got {})",
                chars.len()
            )));
        }

        if chars[8] != 'T' {
            return Err(ParseError(format!(
                "value '{s}' should contain contain a 'T' in the middle, got {})",
                chars[6]
            )));
        }

        Ok(Self(s.to_string()))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Metadata {
    id: Id,
    title: Option<String>,
    slug: String,
    keywords: Vec<String>,
    extension: String,
}

impl Metadata {
    pub fn new(id: Id, title: String, keywords: Vec<String>, extension: String) -> Metadata {
        let slug = slugify(&title);
        Metadata {
            id,
            title: Some(title.clone()),
            slug,
            keywords,
            extension,
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn slug(&self) -> &str {
        self.slug.as_ref()
    }

    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    pub fn extension(&self) -> &str {
        self.extension.as_str()
    }

    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    pub fn front_matter(&self) -> FrontMatter {
        FrontMatter {
            title: self.title.to_owned(),
            date: self.id.human_date(),
            keywords: self.keywords.join(" "),
        }
    }

    pub fn update_title(&mut self, front_matter: &FrontMatter) {
        self.title = front_matter.title.to_owned()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct FrontMatter {
    title: Option<String>,
    date: String,
    keywords: String,
}

impl FrontMatter {
    pub fn dump(&self) -> String {
        serde_yaml::to_string(self).expect("front matter should always be serializable")
    }

    pub fn parse(front_matter: &str) -> Result<Self> {
        serde_yaml::from_str(front_matter).map_err(|e| {
            ParseError(format!(
                "could not deserialize front matter\n{front_matter}\n{e})"
            ))
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Note {
    metadata: Metadata,
    contents: String,
}

impl Note {
    fn relative_path(&self) -> PathBuf {
        let Metadata {
            id,
            slug,
            keywords,
            extension,
            ..
        } = &self.metadata;
        let id = id.as_str();
        let year = &id[0..4];
        let year_path = PathBuf::from_str(year).expect("year should be ascii");

        let trailing_id = &id[4..];
        let keywords = keywords.join("_");

        let file_path =
            PathBuf::from_str(&format!("{trailing_id}--{slug}__{keywords}.{extension}"))
                .expect("filename should be valid utf-8");

        year_path.join(file_path)
    }

    pub fn front_matter(self) -> FrontMatter {
        self.metadata.front_matter()
    }

    pub fn dump(&self) -> String {
        let mut res = String::new();
        // Note: serde_yaml writes a leading `---`
        let front_matter = self.metadata.front_matter();
        res.push_str(&front_matter.dump());
        res.push_str("---\n");
        res.push_str(&self.contents);
        res
    }
}

#[derive(Debug)]
pub struct Notes {
    base_path: PathBuf,
}

impl Notes {
    pub fn try_new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref();
        if !base_path.is_dir() {
            // Note: use ErrorKind::IsADirectory when this variant is
            // stablelized
            return Err(IOError(format!("{base_path:#?} should be a directory")));
        }
        Ok(Notes {
            base_path: base_path.to_owned(),
        })
    }

    pub fn load(&self, relative_path: &Path) -> Result<Note> {
        assert!(relative_path.is_relative());
        let full_path = &self.base_path.join(relative_path);
        let contents = std::fs::read_to_string(full_path)
            .map_err(|e| IOError(format!("While loading note from {full_path:?}: {e}")))?;
        let file_name = &name_from_relative_path(relative_path);
        let metadata = parse_file_name(file_name)?;
        let mut note = Note { metadata, contents };
        if let Some((front_matter, text)) = try_extract_front_matter(&note.contents) {
            note.metadata.title = front_matter.title.to_owned();
            note.contents = text;
        }
        Ok(note)
    }

    pub fn save(&self, note: &Note) -> Result<()> {
        let relative_path = note.relative_path();
        let full_path = &self.base_path.join(relative_path);

        let parent_path = full_path.parent().expect("full path should have a parent");

        if parent_path.exists() {
            if parent_path.is_file() {
                return Err(IOError(format!(
                    "Cannot use {parent_path:?} as year path because there's a file here)"
                )));
            }
        } else {
            println!("Creating {parent_path:?}");
            std::fs::create_dir_all(&parent_path).map_err(|e| {
                IOError(format!(
                    "While creating parent path {parent_path:?}for note :{e}"
                ))
            })?;
        }

        let to_write = note.dump();

        std::fs::write(full_path, &to_write)
            .map_err(|e| IOError(format!("While saving note in {full_path:?}: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use tempfile;

    fn make_note() -> Note {
        let id = Id::from_str("20220707T142708").unwrap();
        let slug = "this-is-a-title".to_owned();
        let title = Some("This is a title".to_owned());
        let keywords = vec!["k1".to_owned(), "k2".to_owned()];
        let extension = "md".to_owned();
        let metadata = Metadata {
            id,
            slug,
            title,
            keywords,
            extension,
        };

        Note {
            metadata,
            contents: "This is my note".to_owned(),
        }
    }

    #[test]
    fn test_slugify_title_when_creating_metadata() {
        let id = Id::from_str("20220707T142708").unwrap();
        let title = "This is a title".to_owned();
        let keywords = vec!["k1".to_owned(), "k2".to_owned()];
        let extension = "md".to_owned();
        let metadata = Metadata::new(id, title, keywords, extension);

        assert_eq!(metadata.slug(), "this-is-a-title");
    }

    #[test]
    fn test_parse_metadata_from_file_name() {
        let name = "20220707T142708--this-is-a-title__k1_k2.md";

        let metadata = parse_file_name(name).unwrap();

        assert_eq!(metadata.id(), "20220707T142708");
        assert_eq!(metadata.slug(), "this-is-a-title");
        assert_eq!(metadata.extension(), "md");
        assert_eq!(metadata.keywords(), &["k1", "k2"]);
    }

    #[test]
    fn test_generate_suitable_file_path_for_note() {
        let note = make_note();
        assert_eq!(
            note.relative_path().to_string_lossy(),
            "2022/0707T142708--this-is-a-title__k1_k2.md"
        );
    }

    #[test]
    fn test_error_when_trying_to_load_notes_from_a_file() {
        Notes::try_new("src/lib.rs").unwrap_err();
    }

    #[test]
    fn test_saving_and_loading() {
        let temp_dir = tempfile::Builder::new()
            .prefix("test-denotes")
            .tempdir()
            .unwrap();
        let notes = Notes::try_new(&temp_dir).unwrap();
        let note = make_note();
        notes.save(&note).unwrap();

        let relative_path = &note.relative_path();
        let saved = notes.load(relative_path).unwrap();
        assert_eq!(note, saved);
    }

    #[test]
    fn test_generating_front_matter() {
        let note = make_note();
        let original = note.front_matter();
        let dumped = original.dump();

        let parsed = FrontMatter::parse(&dumped).unwrap();
        assert_eq!(&parsed.title, &original.title);
    }

    #[test]
    #[ignore]
    fn test_load_front_matter_from_contents() {
        let note = make_note();
        let _contents = note.dump();
    }
}
