#[macro_use]
mod macros;
mod factorio;

fn main() -> anyhow::Result<()> {
    let factorio = factorio::Importer::from("./sample").import()?;
    println!("{:?}", factorio);
    Ok(())
}
