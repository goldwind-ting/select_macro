# select_macro
Waits on multiple concurrent branches, returning when the first branch completes, cancelling the remaining branches. The select! macro must be used inside of async functions, closures, and blocks.

The crate was shamelessly stolen from the [`tokio`][tokio].
More information can be found in the [`docs`][docs].

## License

This project is licensed under the [MIT license].


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Tokio by you, shall be licensed as MIT, without any additional
terms or conditions.



[tokio]: https://github.com/tokio-rs/tokio
[docs]: https://docs.rs/tokio/1.23.0/tokio/macro.select.html#