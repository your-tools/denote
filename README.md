# denote in Rust

## What's denote?

It's a cool project from Protesilaos Stavrou

More details here:

https://protesilaos.com/emacs/denote

## Why a rewrite in Rust

Because I can!

Also Rust may be a better language to built tools on top of it than Lisp

For instance, it should be easy to provide Python or Javascript implementations
of denotes based on this repo.

## Command line usage

See:

```bash
cargo run -- --help
```

## Python bindings

To use the python bindings, make sure you can build the Rust code, then install
poetry and run:

```basd
poetry install
poetry run maturin develop
```


## Kakoune integration

Work in progress


