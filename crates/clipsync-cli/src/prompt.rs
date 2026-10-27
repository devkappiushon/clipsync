//! Line-oriented prompts over any reader/writer, so tests can script them.
use std::io::{self, BufRead, Write};

pub struct Prompter<R: BufRead, W: Write> {
    input: R,
    out: W,
}

impl<R: BufRead, W: Write> Prompter<R, W> {
    pub fn new(input: R, out: W) -> Self {
        Self { input, out }
    }

    /// One trimmed line; `None` at end of input.
    pub fn line(&mut self, question: &str) -> io::Result<Option<String>> {
        write!(self.out, "{question} ")?;
        self.out.flush()?;
        let mut buf = String::new();
        if self.input.read_line(&mut buf)? == 0 {
            writeln!(self.out)?;
            return Ok(None);
        }
        Ok(Some(buf.trim().to_owned()))
    }

    /// `[s/N]` (default no) or `[S/n]` (default yes). End of input or an empty line gives the default.
    pub fn yes_no(&mut self, question: &str, default: bool) -> io::Result<bool> {
        let hint = if default { "[S/n]" } else { "[s/N]" };
        loop {
            let Some(ans) = self.line(&format!("{question} {hint}"))? else { return Ok(default) };
            match ans.to_lowercase().as_str() {
                "" => return Ok(default),
                "s" | "sim" | "y" | "yes" => return Ok(true),
                "n" | "nao" | "não" | "no" => return Ok(false),
                _ => writeln!(self.out, "responda s ou n")?,
            }
        }
    }

    pub fn say(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.out, "{text}")
    }
}
