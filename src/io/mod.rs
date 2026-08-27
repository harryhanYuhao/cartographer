use crate::Graph;
use std::fs::File;
use std::io::{Error, Write};

pub fn export_graph3(g: &Graph, filename: &str) -> Result<(), Error> {
    let mut file = File::create(filename)?;
    writeln!(file, "{}", g.to_graph3())?;
    Ok(())
}
