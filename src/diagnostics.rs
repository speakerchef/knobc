use std::fmt::Display;

use crate::lexer::LocData;
use thiserror::Error;

#[derive(Clone, Copy, Debug)]
pub enum Severity {
    Warn,
    Err,
    Note,
}

const RED: &str = "\x1b[1;93m";
const YELLOW: &str = "\x1b[0;91m";
const PURPLE: &str = "\x1b[1;95m";
const RESET: &str = "\x1b[0m";

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warn => write!(f, "{RED}Warn{RESET}"),
            Severity::Err => write!(f, "{YELLOW}Error{RESET}"),
            Severity::Note => write!(f, "{PURPLE}Note{RESET}"),
        }
    }
}

#[derive(Error, Debug, Clone)]
pub struct Diagnostic {
    severity: Severity,
    loc: LocData,
    msg: String,
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.loc, self.severity, self.msg)
    }
}

#[derive(Debug, Default)]
pub struct DiagHandler {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagHandler {
    pub fn new() -> DiagHandler {
        DiagHandler {
            diagnostics: Vec::<Diagnostic>::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diag| matches!(diag.severity, Severity::Err))
    }

    pub fn push_err(&mut self, loc: LocData, msg: &str) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Err,
            loc,
            msg: String::from(msg),
        });
    }
    pub fn push_warn(&mut self, loc: LocData, msg: &str) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warn,
            loc,
            msg: String::from(msg),
        });
    }
    pub fn push_note(&mut self, loc: LocData, msg: &str) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Note,
            loc,
            msg: String::from(msg),
        });
    }

    pub fn display_diagnostics(&self) {
        for diag in &self.diagnostics {
            match diag.severity {
                Severity::Err => eprintln!("{}", diag),
                _ => println!("{}", diag),
            }
        }
    }
}
