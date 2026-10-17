use std::fmt;
use std::io::{self, Write};

pub(crate) fn write(writer: &mut impl Write, arguments: fmt::Arguments<'_>) -> io::Result<()> {
    match writeln!(writer, "{arguments}") {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}
