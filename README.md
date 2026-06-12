# `is-it-slop`

In my experience, low effort LLM-generated slop Rust projects have three common hallmarks:

- using Rust 2021
- (if using workspaces) using workspace resolver version 2
- using generally outdated versions of dependencies

This simple command-line tool can check for this.

```present cargo run -- -h
CLI for detecting slop smell

Usage: is-it-slop <USER>/<REPO>

Arguments:
  <USER>/<REPO>  on which project shall we SNIFF OUT THE SLOP??

Options:
  -h, --help     Print help
  -V, --version  Print version
```

<p align=center>

![your ai slop bores me](./media/0d5.jpg)

</p>

---

**No artificial intelligence was used in the making of this.**

<a href="https://brainmade.org/">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://brainmade.org/white-logo.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://brainmade.org/black-logo.svg">
  <img alt="brainmade" src="https://brainmade.org/white-logo.svg">
</picture>
</a>
