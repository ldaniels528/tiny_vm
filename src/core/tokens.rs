////////////////////////////////////////////////////////////////////
// tokens module
////////////////////////////////////////////////////////////////////

use serde::{Deserialize, Serialize};

use crate::tokens::Token::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Token {
    AlphaNumeric { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    BackticksQuoted { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    DoubleQuoted { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    Numeric { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    Operator { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    SingleQuoted { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
    Symbol { text: String, start: usize, end: usize, line_number: usize, column_number: usize },
}

impl Token {
    /// creates a new alphanumeric token
    pub fn alpha(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        AlphaNumeric {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new backticks-quoted token
    pub fn backticks(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        BackticksQuoted {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new double-quoted token
    pub fn double_quoted(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        DoubleQuoted {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new numeric token
    pub fn numeric(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        Numeric {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new operator token
    pub fn operator(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        Operator {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new single-quoted token
    pub fn single_quoted(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        SingleQuoted {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// creates a new symbol token
    pub fn symbol(text: String, start: usize, end: usize, line_number: usize, column_number: usize) -> Token {
        Symbol {
            text,
            start,
            end,
            line_number,
            column_number,
        }
    }

    /// Returns the "raw" value of the [Token]
    pub fn get_raw_value(&self) -> String {
        (match &self {
            AlphaNumeric { text, .. }
            | BackticksQuoted { text, .. }
            | DoubleQuoted { text, .. }
            | Numeric { text, .. }
            | Operator { text, .. }
            | SingleQuoted { text, .. }
            | Symbol { text, .. } => text
        }).to_string()
    }

    /// Indicates whether the token is alphanumeric.
    pub fn is_alphanumeric(&self) -> bool {
        match self {
            AlphaNumeric { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is an atom (alphanumeric or backticks-quoted).
    pub fn is_atom(&self) -> bool {
        self.is_alphanumeric() || self.is_backticks_quoted()
    }

    /// Indicates whether the token is a string; which could be backticks-quoted,
    /// double-quoted or single-quoted.
    pub fn is_string(&self) -> bool {
        self.is_double_quoted() || self.is_single_quoted()
    }

    /// Indicates whether the token is a backticks-quoted string.
    pub fn is_backticks_quoted(&self) -> bool {
        match self {
            BackticksQuoted { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is a double-quoted string.
    pub fn is_double_quoted(&self) -> bool {
        match self {
            DoubleQuoted { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is a single-quoted string.
    pub fn is_single_quoted(&self) -> bool {
        match self {
            SingleQuoted { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is numeric.
    pub fn is_numeric(&self) -> bool {
        match self {
            Numeric { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is an operator.
    pub fn is_operator(&self) -> bool {
        match self {
            Operator { .. } => true,
            _ => false
        }
    }

    /// Indicates whether the token is a symbol.
    pub fn is_symbol(&self) -> bool {
        match self {
            Symbol { .. } => true,
            _ => false
        }
    }

    pub fn is_type(&self, variant: &str) -> bool {
        match (self, variant) {
            (AlphaNumeric { .. }, "AlphaNumeric")
            | (BackticksQuoted { .. }, "BackticksQuoted")
            | (DoubleQuoted { .. }, "DoubleQuoted")
            | (Numeric { .. }, "Numeric")
            | (Operator { .. }, "Operator")
            | (SingleQuoted { .. }, "SingleQuoted")
            | (Symbol { .. }, "Symbol") => true,
            _ => false,
        }
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use crate::tokens::Token;

    use super::*;

    #[test]
    fn test_is_alphanumeric() {
        assert!(Token::alpha("World".into(), 11, 16, 1, 13).is_alphanumeric());
    }

    #[test]
    fn test_is_atom() {
        assert!(Token::alpha("x".into(), 11, 16, 1, 13).is_atom());
        assert!(Token::backticks("x".into(), 11, 16, 1, 13).is_atom());
    }

    #[test]
    fn test_is_string_backticks() {
        assert!(Token::backticks("the".into(), 0, 3, 1, 2).is_backticks_quoted());
    }

    #[test]
    fn test_is_numeric() {
        assert!(Token::numeric("123".into(), 0, 3, 1, 2).is_numeric());
    }

    #[test]
    fn test_is_operator() {
        assert!(Token::operator(".".into(), 0, 3, 1, 2).is_operator());
    }

    #[test]
    fn test_is_string() {
        assert!(Token::double_quoted("the".into(), 0, 3, 1, 2).is_string());
        assert!(Token::single_quoted("little".into(), 0, 3, 1, 2).is_string());
        assert!(Token::double_quoted("red".into(), 0, 3, 1, 2).is_double_quoted());
        assert!(Token::single_quoted("fox".into(), 0, 3, 1, 2).is_single_quoted());
    }

    #[test]
    fn test_is_symbol() {
        assert!(Token::symbol(",".into(), 3, 4, 1, 5).is_symbol());
    }

    #[test]
    fn test_is_type() {
        assert!(Token::alpha("World".into(), 11, 16, 1, 13).is_type("AlphaNumeric"));
        assert!(Token::backticks("`World`".into(), 11, 16, 1, 13).is_type("BackticksQuoted"));
        assert!(Token::double_quoted("\"World\"".into(), 11, 16, 1, 13).is_type("DoubleQuoted"));
        assert!(Token::numeric("123".into(), 0, 3, 1, 2).is_type("Numeric"));
        assert!(Token::operator(".".into(), 0, 3, 1, 2).is_type("Operator"));
        assert!(Token::single_quoted("'World'".into(), 11, 16, 1, 13).is_type("SingleQuoted"));
        assert!(Token::symbol("÷".into(), 3, 4, 1, 5).is_type("Symbol"));
    }
}