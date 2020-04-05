## jrni

An over-under-engineered journaling tool written in Rust. To be honest, this
should probably just be a bash script. But hey, I wanted to learn Rust.

Jrni maintains a flat directory of raw text journal entries that have a basic
frontmatter structure that includes publication date, tags, and title. The CLI
allows creation and basic querying of tags and related counts.

The nice thing about raw text is that it's grepable, but having some semblance
of structure in the frontmatter allows us to easily generate aggregates from
the entries.

Each post has an optional unique identifier. The `id` subcommand can be used
to quickly edit the entry with a given id.

```
$ ./target/debug/jrni --help

jrni 1.0

USAGE:
    jrni [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --path <DIR>    path to the journal contents directory

SUBCOMMANDS:
    help    Prints this message or the help of the given subcommand(s)
    id      query for id
    n       create a new entry
    t       get a listing of tags with associated entry count
```

Here's an example usage:

```
$ jrni n whoa
[ ... $EDITOR opens ...]
/home/james/sink/journal/2020-04-05-whoa.md

$ cat /home/james/sink/journal/2020-04-05-whoa.md
tags: test,readme
id: whoa
pubdate: 2020-04-05 12:41:17.111 -0400
---

Pretty simple, eh?
```

### Installation

`cargo install --path .`, then ensure `~/.cargo/bin` is on your `PATH`.

### Configuration

The following environment variables are respected:
- `EDITOR`: controls which editor jrni uses to edit posts
- `JRNI_PATH`: a path to the folder containing journal entries
