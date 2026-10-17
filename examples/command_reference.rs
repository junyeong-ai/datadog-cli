use clap::{Command, CommandFactory};
use datadog_cli::cli::Cli;
use std::io::{self, Write};

fn render(mut command: Command, path: &str, output: &mut impl Write) -> io::Result<()> {
    let children: Vec<Command> = command.get_subcommands().cloned().collect();
    command = command.bin_name(path);
    command.build();
    writeln!(
        output,
        "## `{path}`\n\n```text\n{}\n```\n",
        command
            .render_help()
            .to_string()
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
    )?;
    for child in children {
        let path = format!("{path} {}", child.get_name());
        render(child, &path, output)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "# Datadog CLI command reference\n\nGenerated from Clap with `cargo run --locked --example command_reference`.\nGlobal options work before or after subcommands. JSON retains pagination metadata; JSONL emits data records only.\n"
    )?;
    render(Cli::command(), "datadog-cli", &mut output)?;
    writeln!(output, "## Supported sites\n")?;
    for site in datadog_cli::config::VALID_SITES {
        writeln!(output, "- `{site}`")?;
    }
    output.flush()
}
