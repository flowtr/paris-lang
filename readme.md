# ParisLang

A programming language written in Rust.

## Syntax

Inspired by Ruby, Javascript and Go, you can create your first variable by using the `:=` operator.

```paris
x := `hello world`
```

Now you can print it to your terminal.

```paris
display x
```

## Building from Source

In order to build ParisLang, you will need the latest [rust nightly version installed on your machine](https://rustup.rs).

You can compile it with `cargo build --release`, and the executable will be located at `target/release/paris-lang`.

