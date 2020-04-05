/// This module maintains a database of entries based on a flat filesystem.
///
/// Each entry has Jekyll-style frontmatter associated with it that include
/// the date of the post, associated tags, and a title.
///
/// There is an overengineered parallelized mechanism for walking the journal
/// contents, `walk_journal`.
///
use std::sync::mpsc::channel;
use std::mem::drop;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::fs;
use std::collections::HashMap;
use std::io::{self, BufRead};

use threadpool::ThreadPool;
use walkdir::{WalkDir, DirEntry};
use serde_yaml::Value as YValue;

use crate::error::Result;


#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub file_metadata: fs::Metadata,
    pub frontmatter: HashMap<String, YValue>,

    /// If an error was encountered while trying to decode frontmatter, 
    /// attach it here.
    pub frontmatter_err: Option<serde_yaml::Error>,
    pub body: String,
}

impl Entry {
    pub fn from_path(p: &Path) -> Result<Entry> {
        let pathstr = p.to_str().unwrap();
        let mut fm = HashMap::new();
        let mut fm_err = None;
        let mut all_lines: Vec<String> = Vec::new();
        let mut rawfrontmatter: Vec<String> = Vec::new();
        let mut body: Vec<String> = Vec::new();
        let mut into = &mut rawfrontmatter;
        let mut frontmatter_end_idx = -1;
        let mut idx = 0;

        let f = io::BufReader::new(fs::File::open(&pathstr)?);

        for line in f.lines() {
            let line = line?;
            all_lines.push(line.clone());
            idx += 1;

            if frontmatter_end_idx == -1 && line.trim() == "---" {
                into = &mut body;
                frontmatter_end_idx = idx;
            } else {
                into.push(line);
            }
        }

        if frontmatter_end_idx != -1 {
            let yaml_result = serde_yaml::from_str(&rawfrontmatter.join("\n"));
            match yaml_result {
                Err(yaml_err) => fm_err = Some(yaml_err),
                Ok(res) => fm = res,
            }
        } 
            
        if let Some(_) = fm_err {
            body = all_lines;
        }

        // In case no frontmatter is attached.
        if body.len() == 0 {
            body = rawfrontmatter;
        }

        fm.insert("tags".to_owned(), normalize_tags(fm.get("tags")));

        Ok(Entry {
            path: p.to_owned(),
            file_metadata: fs::metadata(pathstr)?,
            frontmatter: fm,
            frontmatter_err: fm_err,
            body: body.join("\n"),

        })
    }

    pub fn get_tags(&self) -> Option<Vec<&str>> {
        if !self.frontmatter.contains_key("tags") {
            return None;
        }

        let mut tags = Vec::new();

        for t in self.frontmatter.get("tags").unwrap().as_sequence().unwrap() {
            tags.push(t.as_str().unwrap());
        }

        Some(tags)
    }

    pub fn get_id(&self) -> Option<&str> {
        if !self.frontmatter.contains_key("id") {
            return None;
        }
        
        let id = self.frontmatter.get("id")?.as_str()?;
        match id.len() { 0 => None, _ => Some(id) }
    }
}

fn normalize_tags(tags: Option<&YValue>) -> YValue {
    match tags {
        Some(val) => match val {
            YValue::String(v) => {
                let split: Vec<YValue> = v.split(",").map(
                    |s| YValue::String(s.trim().to_owned())).collect();
                YValue::Sequence(split)
            },
            YValue::Sequence(_) => val.to_owned(),
            YValue::Null => YValue::Sequence(Vec::new()),
            _ => {
                YValue::Sequence(Vec::new())
                // TODO log bad tags
            },
        },
        None => YValue::Sequence(Vec::<YValue>::new()),
    }
}

/// Ignore paths that don't end in extensions we can make sense of.
///
fn is_jrnl_path(p: &Path) -> bool {
    if p.is_dir() {
        return false;
    }
    match p.extension() {
        Some(osstr) =>
            match osstr.to_str().unwrap() {
                "md" | "txt" => return true,
                _ => false
            }
        _ => false,
    }
}

fn get_jrnl_walker(jrnl_path: &str) -> Box<Iterator<Item = DirEntry>> {
    Box::new(WalkDir::new(jrnl_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_jrnl_path(e.path())))
}


/// For each entry in the journal, perform some action per `path_fn` and
/// return a vector of the results. 
///
/// This happens in parallel using a threadpool.
///
pub fn walk_journal<T, F>(jrnl_path: &str, path_fn: F) -> Vec<Result<T>>
    where F : Fn(PathBuf) -> Result<T> + Send + Sync + 'static, 
        T : Send + 'static
{
    let pool = ThreadPool::new(num_cpus::get());
    let (tx, rx) = channel();
    let fn_ref = Arc::new(path_fn);

    for entry in get_jrnl_walker(jrnl_path) {
        let path = entry.path().to_owned();
        let tx = tx.clone();
        let path_fn = fn_ref.clone();

        pool.execute(move || {
            tx.send(path_fn(path)).expect("Couldn't send data!");
        });
    }

    drop(tx);
    rx.iter().collect()
}      
