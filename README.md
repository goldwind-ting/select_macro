# select_macro
<a href="https://crates.io/crates/select-macro">
    <img src="https://img.shields.io/crates/v/select-macro.svg?style=flat-square"
    alt="Crates.io version" />
</a> 
<a href="https://crates.io/crates/select-macro">
    <img src="https://img.shields.io/crates/d/select-macro.svg?style=flat-square"
      alt="Download" />
</a>


Waits on multiple concurrent branches, returning when the first branch completes, cancelling the remaining branches. The select! macro must be used inside of async functions, closures, and blocks.

The crate was shamelessly stolen from the [`tokio`][tokio].
More information can be found in the [`docs`][docs].

## Motivation
Refer to the [issue](https://github.com/tokio-rs/tokio/issues/5312).

## Usage
Add this to your Cargo.toml

```toml
[dependencies]
select-macro = "0.2.0"
```

## Quick start:

```rust
use async_std::{channel as async_channel, stream::{self, StreamExt}, task};
use std::time::Duration;
use select_macro::{count, select, select_variant};

#[async_std::main]
async fn main(){
    let mut inter = stream::interval(Duration::from_secs(2));
    let (rx, tx) = async_channel::bounded(1);
    task::spawn(async move{
            task::sleep(Duration::from_secs(1)).await;
            rx.send(1).await.unwrap();
    });
    select!{
        _ = inter.next() => {
            panic!("unreachable!");
        }
        data = tx.recv() => {
            assert_eq!(data, Ok(1));
        }
    };
}
```


## License

This project is licensed under the [MIT license].


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in select-macro by you, shall be licensed as MIT, without any additional
terms or conditions.



[tokio]: https://github.com/tokio-rs/tokio
[docs]: https://docs.rs/tokio/1.23.0/tokio/macro.select.html#