use std::io::{self, Write};

pub fn write(content: String) -> io::Result<()> {
    io::stdout().write(content.as_bytes())?;

    Ok(())
}

pub fn write_err(content: String) -> io::Result<()> {
    io::stderr().write(content.as_bytes())?;

    Ok(())
}
