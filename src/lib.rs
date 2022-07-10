use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::macros::format_description;
use time::OffsetDateTime;

/// Tools for command-line usage
pub mod cli;

/// Python bindings
mod python;

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
/// Variant of the errors returned by the libray
#[non_exhaustive]
pub enum Error {
    #[error("parse error")]
    ParseError(String),
    #[error("os error")]
    OSError(String),
}

use Error::*;

/// Result type for this library
pub type Result<T> = std::result::Result<T, Error>;

fn name_from_relative_path(relative_path: &Path) -> String {
    let components: Vec<_> = relative_path.components().collect();
    assert!(
        components.len() >= 2,
        "relative path should look like <year>/<id>"
    );
    let last: &Path = components
        .last()
        .expect("components cannot be empty")
        .as_ref();
    last.to_string_lossy().into_owned()
}

pub struct FileInfo {
    pub id: Id,
    pub slug: String,
    pub keywords: Vec<String>,
    pub extension: String,
}

pub fn parse_file_name(name: &str) -> Result<FileInfo> {
    let captures = FILENAME_RE
        .captures(name)
        .ok_or_else(|| ParseError(format!("Filename {name} did not match expected regex")))?;

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

    Ok(FileInfo {
        id,
        slug,
        keywords,
        extension,
    })
}

fn parse_front_matter(contents: &str) -> Result<(FrontMatter, String)> {
    let docs: Vec<_> = contents.splitn(3, "---\n").collect();
    if docs.is_empty() {
        return Err(Error::ParseError("Missing front matter".to_string()));
    }
    if docs.len() < 2 {
        return Err(Error::ParseError("Unfinished front matter".to_string()));
    }
    let first_doc = &docs[1];
    let text = docs[2];
    let front_matter = FrontMatter::parse(first_doc)?;
    Ok((front_matter, text.to_string()))
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Deserialize, Serialize)]
/// A new-type on top of String so that only valid Ids can
/// be used
/// As a reminder, the Id in denote is YYYYMMDDTHHMMSS
pub struct Id(String);

impl Id {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn human_date(&self) -> String {
        let ymd = &self.0[0..8];
        let year = &ymd[0..4];
        let month = &ymd[4..6];
        let day = &ymd[6..8];

        let hms = &self.0[9..];
        let hours = &hms[0..2];
        let minutes = &hms[2..4];
        let seconds = &hms[4..6];

        format!("{year}-{month}-{day} {hours}:{minutes}:{seconds}")
    }

    pub fn from_date(offsett_date_time: &OffsetDateTime) -> Self {
        let format = format_description!("[year][month][day]T[hour][minute][second]");
        let formatted_date = offsett_date_time.format(&format).unwrap();
        Self::from_str(&formatted_date).unwrap()
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
/// Contain all the metadata about a note.
/// Some of it come from the front matter, like the title,
/// but some other come from the filename, like the slug, the extension,
/// or the keywords
pub struct Metadata {
    id: Id,
    title: String,
    slug: String,
    keywords: Vec<String>,
    extension: String,
}

impl Metadata {
    pub fn new(id: Id, title: String, keywords: Vec<String>, extension: String) -> Metadata {
        let slug = slug::slugify(&title);
        Metadata {
            id,
            title,
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

    pub fn title(&self) -> &str {
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

    pub fn relative_path(&self) -> PathBuf {
        let Metadata {
            id,
            keywords,
            slug,
            extension,
            ..
        } = self;

        let id = id.as_str();
        let year = &id[0..4];
        let year_path = PathBuf::from_str(year).expect("year should be ascii");

        let keywords = keywords.join("_");

        let file_path = PathBuf::from_str(&format!("{id}--{slug}__{keywords}.{extension}"))
            .expect("filename should be valid utf-8");

        year_path.join(file_path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
/// The front matter of a note.
/// Currently using YAML
/// Note that `keywords` is list of words separated by spaces,
/// which is find because we don't allow spaces in keywords.
///
/// The title may not be set
pub struct FrontMatter {
    title: String,
    date: String,
    keywords: String,
}

impl FrontMatter {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn keywords(&self) -> Vec<String> {
        self.keywords.split(' ').map(|x| x.to_string()).collect()
    }

    pub fn dump(&self) -> String {
        serde_yaml::to_string(self).expect("front matter should always be serializable")
    }

    pub fn slug(&self) -> String {
        slug::slugify(&self.title)
    }

    pub fn parse(front_matter: &str) -> Result<Self> {
        serde_yaml::from_str(front_matter).map_err(|e| {
            ParseError(format!(
                "could not deserialize front matter\n{front_matter}\n{e})"
            ))
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
/// A Note has some metadata and some text
/// Note that the metada is different from the frontmatter, it does
/// contain exacly the same data
pub struct Note {
    metadata: Metadata,
    text: String,
}

impl Note {
    pub fn new(metadata: Metadata, text: String) -> Self {
        Self { metadata, text }
    }

    fn relative_path(&self) -> PathBuf {
        self.metadata.relative_path()
    }

    pub fn front_matter(&self) -> FrontMatter {
        self.metadata.front_matter()
    }

    /// Update the metadata when the front matter changes
    pub fn update(&mut self, front_matter: &FrontMatter) {
        self.metadata.title = front_matter.title.to_string();
        self.metadata.slug = front_matter.slug();
        self.metadata.keywords = front_matter.keywords();
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn id(&self) -> &str {
        self.metadata.id()
    }

    pub fn dump(&self) -> String {
        let mut res = String::new();
        // Note: serde_yaml writes a leading `---`
        let front_matter = self.metadata.front_matter();
        res.push_str(&front_matter.dump());
        res.push_str("---\n");
        res.push_str(&self.text);
        res
    }
}

fn get_note_from_markdown(id: Id, contents: String) -> Result<Note> {
    let (front_matter, text) = parse_front_matter(&contents)?;
    let title = front_matter.title.to_string();
    let slug = front_matter.slug();
    let keywords = front_matter.keywords();
    let metadata = Metadata {
        id,
        title,
        slug,
        keywords,
        extension: "md".to_string(),
    };
    Ok(Note { metadata, text })
}

#[derive(Debug)]
/// Store the notes with the proper file names inside a `base_path`
pub struct NotesRepository {
    base_path: PathBuf,
}

impl NotesRepository {
    /// Open a new repository given a base_path
    /// Base path should contain one folder per year,
    /// and the filename in each `<year>`` folder should match
    /// the denote naming convention
    pub fn open(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref();
        if !base_path.is_dir() {
            // Note: use ErrorKind::IsADirectory when this variant is
            // stablelized
            return Err(OSError(format!("{base_path:#?} should be a directory")));
        }
        Ok(NotesRepository {
            base_path: base_path.to_owned(),
        })
    }

    /// The base path of the repository, where the `<year>` directories
    /// are created
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Import a plain md file and save it with the correct name
    /// Called by cli::new_note
    pub fn import_from_markdown(&self, markdown_path: &Path) -> Result<PathBuf> {
        let contents = std::fs::read_to_string(markdown_path)
            .map_err(|e| Error::OSError(format!("while reading: {markdown_path:#?}: {e}")))?;
        let now = OffsetDateTime::now_utc();
        let id = Id::from_date(&now);

        let note = get_note_from_markdown(id, contents)
            .map_err(|e| Error::OSError(format!("invalid contents for {markdown_path:#?}: {e}")))?;
        self.save(&note)
    }

    /// To be called when the markdown file has changed - this will
    /// handle the rename automatically - note that the ID won't change,
    /// this is by design
    /// Return the new note path (which may have changed)
    pub fn update(&self, relative_path: &Path) -> Result<PathBuf> {
        let full_path = &self.base_path.join(relative_path);
        let note = self.load(relative_path)?;

        let new_relative_path = note.relative_path();
        let new_full_path = &self.base_path.join(&new_relative_path);
        if full_path != new_full_path {
            println!("{full_path:#?} -> {new_full_path:#?}");
            std::fs::rename(full_path, new_full_path)
                .map_err(|e| Error::OSError(format!("Could not rename note: {e}")))?;
        }

        Ok(new_full_path.to_path_buf())
    }

    /// Load a note file
    pub fn load(&self, relative_path: &Path) -> Result<Note> {
        if !relative_path.is_relative() {
            return Err(OSError(format!(
                "Expecting a relative path when loading, get {relative_path:+?}"
            )));
        }
        let full_path = &self.base_path.join(relative_path);
        let contents = std::fs::read_to_string(full_path)
            .map_err(|e| OSError(format!("While loading note from {full_path:?}: {e}")))?;

        let file_name = &name_from_relative_path(relative_path);
        let info = parse_file_name(file_name)?;
        get_note_from_markdown(info.id, contents)
    }

    /// Save a note in the repository
    /// Create `<year>` directory when needed
    pub fn save(&self, note: &Note) -> Result<PathBuf> {
        let relative_path = &note.relative_path();
        let full_path = &self.base_path.join(relative_path);

        let parent_path = full_path.parent().expect("full path should have a parent");

        if parent_path.exists() {
            if parent_path.is_file() {
                return Err(OSError(format!(
                    "Cannot use {parent_path:?} as year path because there's a file here)"
                )));
            }
        } else {
            println!("Creating {parent_path:?}");
            std::fs::create_dir_all(&parent_path).map_err(|e| {
                OSError(format!(
                    "While creating parent path {parent_path:?}for note :{e}"
                ))
            })?;
        }

        let to_write = note.dump();

        std::fs::write(full_path, &to_write)
            .map_err(|e| OSError(format!("While saving note in {full_path:?}: {e}")))?;
        Ok(relative_path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_id_sorting() {
        let id1 = Id::from_str("20220707T142708").unwrap();
        let id2 = Id::from_str("2022070lT142709").unwrap();
        let id3 = Id::from_str("2022070lT142709").unwrap();

        assert_eq!(id2, id3);
        assert!(id1 < id2)
    }

    fn make_note() -> Note {
        let id = Id::from_str("20220707T142708").unwrap();
        let slug = "this-is-a-title".to_owned();
        let title = "This is a title".to_owned();
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
            text: "This is my note".to_owned(),
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
    fn test_parse_info_from_file_name() {
        let name = "20220707T142708--this-is-a-title__k1_k2.md";

        let file_info = parse_file_name(name).unwrap();

        assert_eq!(file_info.id.as_str(), "20220707T142708");
        assert_eq!(file_info.slug.as_str(), "this-is-a-title");
        assert_eq!(&file_info.keywords, &["k1", "k2"]);
        assert_eq!(file_info.extension.as_str(), "md");
    }

    #[test]
    fn test_generate_suitable_file_path_for_note() {
        let note = make_note();
        assert_eq!(
            note.relative_path().to_string_lossy(),
            "2022/20220707T142708--this-is-a-title__k1_k2.md"
        );
    }

    #[test]
    fn test_error_when_trying_to_load_notes_from_a_file() {
        NotesRepository::open("src/lib.rs").unwrap_err();
    }

    #[test]
    fn test_saving_and_loading() {
        let temp_dir = tempfile::Builder::new()
            .prefix("test-denotes")
            .tempdir()
            .unwrap();
        let notes = NotesRepository::open(&temp_dir).unwrap();
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
