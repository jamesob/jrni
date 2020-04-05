#[macro_use]
extern crate error_chain;

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write, stdin};
use std::env;

use clap::{Arg, App, SubCommand};
use jrni::{Entry, walk_journal, datetime};
use jrni::error::*;


fn run() -> Result<()> {
    let new_sub = SubCommand::with_name("n")
        .about("create a new entry")
        .arg(Arg::from_usage("-t --tags=[tags] 'tags to apply'"))
        .arg(Arg::from_usage("--stdin 'read body from stdin'"))
        .arg(Arg::from_usage("<entryname> 'filename of the entry'"));
                             
    let tags_sub = SubCommand::with_name("t")
        .about("get a listing of tags with associated entry count");

    let id_sub = SubCommand::with_name("id")
        .about("query for id")
        .arg(Arg::from_usage("[id] 'if specified, edit the file with this shortname'"));
                             
    let matches = App::new("jrni")
        .version("1.0")
        .arg(Arg::with_name("path")
             .short("p")
             .long("path")
             .value_name("DIR")
             .help("path to the journal contents directory")
             .takes_value(true))
        .subcommand(new_sub)
        .subcommand(tags_sub)
        .subcommand(id_sub)
        .get_matches();

    // Take the journal path from
    //
    //   - `-p`, the path argument, or
    //   - JRNI_PATH, the environment variable, or
    //   - default to `~/sink/journal`, which is probably relevant for no one 
    //     but me.
    //
    let default_path = dirs::home_dir().unwrap().join("sink/journal");
    let path = match matches.value_of("path") {
        Some(v) => String::from(v),
        None => match env::var_os("JRNI_PATH") {
            Some(v2) =>  v2.into_string().unwrap(),
            None => String::from(default_path.to_str().unwrap()),
        }
    };
    let mut path = PathBuf::from(path);
    
    let res: Result<_> = match matches.subcommand() {
        ("n", Some(sub_m)) => new_entry(
            &mut path, 
            sub_m.value_of("entryname").unwrap(),
            sub_m.value_of("tags"),
            sub_m.is_present("stdin"),
            ),
        ("t", Some(_)) => query_tags(path),
        ("id", Some(sub_m)) => {
            if sub_m.is_present("id") {
                edit_by_id(path, sub_m.value_of("id").unwrap())
            } else {
                query_ids(path)
            }
        }
        (&_, _) => Ok(()),
    };

    res
}

quick_main!(run);
 
fn get_entries(files_path: &PathBuf) -> impl Iterator<Item = Entry> {
    walk_journal(
        &files_path.to_str().unwrap(), 
        |p| Entry::from_path(&p)
    )
        .into_iter()
        .filter_map(|e| match e {
            Ok(e) => Some(e),
            Err(_) => { None // TODO error log 
            },
        })
}

fn edit(path: &str) {
    let editor = match env::var_os("EDITOR") {
        Some(v) => v.into_string().unwrap(),
        // Fall back to "nvim" for the default editor.
        None => String::from("nvim"),
    };
    Command::new(editor)
        .arg(path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("uhoh");
    println!("{}", path); 
}

/// Crate a new entry, populating it with front matter, and open $EDITOR.
///
/// Optionally populate it with input from stdin.
///
pub fn new_entry(
    files_path: &mut PathBuf, 
    name: &str, 
    tags: Option<&str>,
    read_body_from_stdin: bool,
) -> Result<()> {
    let now = datetime::now();
    let filename = format!("{}-{}.md", now.format("%F"), name);
    files_path.push(filename);

    let mut body = String::new();
    if read_body_from_stdin {
        stdin().read_to_string(&mut body)?;
    }
     
    let pathstr = files_path.to_str().unwrap();
 
    if files_path.exists() {
        bail!("file with path {} already exists", pathstr);
    }                       

    let entries: Vec<Entry> = get_entries(files_path).collect();
    let existing_ids: HashSet<&str> = entries.iter()
        .filter_map(|e| e.get_id()).collect();

    let id = match existing_ids.contains(name) {
        true => "",
        false => name,
    };

    let mut output = File::create(pathstr)?;
    let contents =  format!(
        "tags: {}\nid: {}\npubdate: {}\n---\n\n{}\n", 
        tags.unwrap_or(""), id, datetime::to_str(now), body);
    write!(output, "{}", contents).expect("write failed");
    edit(pathstr);
    Ok(())
}

/// Print tags sorted by related entry count.
/// 
pub fn query_tags(files_path: PathBuf) -> Result<()> {
    let entries = get_entries(&files_path);
    let mut counts: HashMap<String, i32> = HashMap::new();

    for e in entries {
        if let Some(tags) = e.get_tags() {
            for t in tags.into_iter() {
                *counts.entry(t.to_owned()).or_insert(0) += 1;
            }
        }        
    }

    let mut sorted: Vec<(String, i32)> = counts.into_iter().collect();

    sorted.sort_unstable_by_key(|v| v.1);

    for (tag, count) in sorted.iter() {
        println!("{} {}", tag, count);
    }

    Ok(())
}

pub fn edit_by_id(files_path: PathBuf, id: &str) -> Result<()> {
    let entries = get_entries(&files_path);

    for e in entries.into_iter() {
        if let Some(e_id) = e.get_id() {
            if e_id == id {
                edit(e.path.to_str().unwrap());
                return Ok(());
            }
        }
    }

    println!("Couldn't find entry by id '{}'", id);
    Ok(())
}

/// Print the id associated with each entry.
///
pub fn query_ids(files_path: PathBuf) -> Result<()> {
    for e in get_entries(&files_path).into_iter() {
        if let Some(id) = e.get_id() {
            println!("{}", id);
        }
    }
    Ok(())
}
