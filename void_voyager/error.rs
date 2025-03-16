use miette::Diagnostic;
use snafu::prelude::*;

#[derive(Debug, Snafu, Diagnostic)]
pub enum Error {
    #[snafu(display("Failed to bind to address {}", address))]
    #[diagnostic(code(void_voyager::main::main), help("Is the port available?"))]
    Bind {
        address: String,
        source: std::io::Error,
    },
    #[snafu(display("Failed to serve Voyager app"))]
    #[diagnostic(
        code(void_voyager::main::main),
        help("You're on your own for this one.")
    )]
    Serve {
        source: std::io::Error,
    },
    #[snafu(display("Parsing error occured: {}", source))]
    #[diagnostic(
        code(void_codex::sector::Compendium::decipher),
        help("Is it an issue with the parser, or with the level?")
    )]
    Parse {
        source: void_codex::error::Error,
    },
    ReadAtlas {
        source: Box<bincode::ErrorKind>,
    },
    ReadManifest {
        source: ron::error::SpannedError,
    },
}

impl From<void_codex::error::Error> for Error {
    fn from(value: void_codex::error::Error) -> Self {
        Self::Parse { source: value }
    }
}
