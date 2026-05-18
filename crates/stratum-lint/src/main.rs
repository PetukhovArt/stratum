#![forbid(unsafe_code)]

fn main() -> miette::Result<()> {
    println!("stratum-lint {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
