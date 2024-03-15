////////////////////////////////////////////////////////////////////
// token slice module - responsible for parsing language tokens
////////////////////////////////////////////////////////////////////

use std::ops::Index;

use serde::{Deserialize, Serialize};

use crate::tokenizer;
use crate::tokens::Token;

/// TokenSlice is a navigable sequence of tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenSlice {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenSlice {
    ////////////////////////////////////////////////////////////////
    // static methods
    ////////////////////////////////////////////////////////////////

    /// Creates a new Token Slice via a vector of tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Creates a new Token Slice from a string.
    pub fn from_string(text: &str) -> Self {
        Self::new(tokenizer::parse_fully(text))
    }

    ////////////////////////////////////////////////////////////////
    // instance methods
    ////////////////////////////////////////////////////////////////

    pub fn capture(&mut self, start: &str, end: &str, delim: Option<&str>) -> Vec<Token> {
        let inputs = &self.tokens;
        println!("capture: {:?}", inputs);
        let mut pos = self.pos;
        let mut tokens = vec![];
        if inputs[pos].get_raw_value() == start {
            pos += 1;
            while (pos < inputs.len()) && inputs[pos].get_raw_value() != end {
                tokens.push(inputs[pos].clone());
                pos += 1;
                match delim {
                    Some(_delim) if pos < inputs.len() && inputs[pos].get_raw_value() == _delim => pos += 1,
                    _ => {}
                }
            }
        }
        self.pos = pos;
        tokens
    }

    pub fn exists(&self, f: fn(&Token) -> bool) -> bool {
        match self.get() {
            Some(token) => f(token),
            None => false
        }
    }

    /// Returns the option of a Token at the current position within the slice.
    pub fn get(&self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            return Some(&self.tokens[self.pos]);
        }
        None
    }

    /// Returns the position (index) within the slice.
    pub fn get_position(&self) -> usize { self.pos }

    /// Indicates whether at least one more token remains before the end of the slice.
    pub fn has_next(&self) -> bool { self.pos + 1 < self.tokens.len() }

    /// Indicates whether at least one more token remains before the beginning of the slice.
    pub fn has_previous(&self) -> bool { self.pos > 0 }

    pub fn is(&self, text: &str) -> bool {
        match self.get() {
            Some(tok) => tok.get_raw_value() == text,
            None => false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Returns the option of a Token at the next position within the slice.
    pub fn next(&mut self) -> Option<&Token> {
        if self.has_next() {
            let n = self.pos + 1;
            self.pos = n;
            return Some(&self.tokens[n]);
        }
        None
    }

    /// Returns the option of a Token at the previous position within the slice.
    pub fn previous(&mut self) -> Option<&Token> {
        if self.has_previous() {
            let n = self.pos - 1;
            self.pos = n;
            return Some(&self.tokens[n]);
        }
        None
    }

    /// Scans the slice moving the cursor forward until the desired match is found.
    /// However, if the end of the sequence is reached before the token is found
    /// there is no effect.
    pub fn scan_to(&mut self, f: fn(&Token) -> bool) -> &[Token] {
        let mut pos = self.pos;
        while pos < self.tokens.len() && !f(&self.tokens[pos]) { pos += 1 }
        if pos > self.pos && pos < self.tokens.len() {
            let result = &self.tokens[self.pos..pos];
            self.pos = pos;
            result
        } else { &[] }
    }

    /// Scans the slice moving the cursor forward until the desired match is found.
    /// However, if the end of the sequence is reached before the token is found
    /// there is no effect.
    pub fn scan_until(&mut self, f: fn(&Token) -> bool) -> &[Token] {
        let mut pos = self.pos;
        while pos < self.tokens.len() && !f(&self.tokens[pos]) { pos += 1 }
        if pos > self.pos && pos < self.tokens.len() {
            let result = &self.tokens[self.pos..=pos];
            self.pos = pos;
            result
        } else { &[] }
    }

    /// Returns the current token; then moves the cursor forward.
    pub fn take(&mut self) -> Option<&Token> {
        let n = self.pos;
        if n < self.tokens.len() {
            let tok = Some(&self.tokens[n]);
            self.pos = n + 1;
            tok
        } else { None }
    }
}

impl Index<usize> for TokenSlice {
    type Output = Token;

    fn index(&self, index: usize) -> &Self::Output {
        &self.tokens[index]
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_with_delimiter() {
        let mut ts = TokenSlice::from_string("(123, 'Hello', abc)");
        let tokens = ts.capture("(", ")", Some(","));
        assert_eq!(tokens, vec![
            Token::numeric("123".into(), 1, 4, 1, 3),
            Token::single_quoted("'Hello'".into(), 6, 13, 1, 8),
            Token::alpha("abc".into(), 15, 18, 1, 17),
        ])
    }

    #[test]
    fn test_capture_without_delimiter() {
        let mut ts = TokenSlice::from_string("(123, 'Hello', abc)");
        let tokens = ts.capture("(", ")", None);
        assert_eq!(tokens, vec![
            Token::numeric("123".into(), 1, 4, 1, 3),
            Token::symbol(",".into(), 4, 5, 1, 6),
            Token::single_quoted("'Hello'".into(), 6, 13, 1, 8),
            Token::symbol(",".into(), 13, 14, 1, 15),
            Token::alpha("abc".into(), 15, 18, 1, 17),
        ])
    }

    #[test]
    fn test_cursor_current_next_and_previous() {
        let mut ts = TokenSlice::from_string("123, Hello World");
        assert_eq!(ts.get(), Some(&Token::numeric("123".into(), 0, 3, 1, 2)));
        assert_eq!(ts.take(), Some(&Token::numeric("123".into(), 0, 3, 1, 2)));
        assert_eq!(ts.take(), Some(&Token::symbol(",".into(), 3, 4, 1, 5)));
        assert_eq!(ts.take(), Some(&Token::alpha("Hello".into(), 5, 10, 1, 7)));
        assert_eq!(ts.take(), Some(&Token::alpha("World".into(), 11, 16, 1, 13)));
        assert_eq!(ts.take(), None);
        assert_eq!(ts.previous(), Some(&Token::alpha("World".into(), 11, 16, 1, 13)));
        assert_eq!(ts.previous(), Some(&Token::alpha("Hello".into(), 5, 10, 1, 7)));
    }

    #[test]
    fn test_exists() {
        let ts = TokenSlice::from_string("'Hello World'");
        assert!(ts.exists(|t| t.is_single_quoted()));
    }

    #[test]
    fn test_from_string() {
        let ts = TokenSlice::from_string("123 Hello World");
        assert_eq!(ts, TokenSlice {
            tokens: vec![
                Token::numeric("123".into(), 0, 3, 1, 2),
                Token::alpha("Hello".into(), 4, 9, 1, 6),
                Token::alpha("World".into(), 10, 15, 1, 12),
            ],
            pos: 0,
        })
    }

    #[test]
    fn test_indexing_into() {
        let ts = TokenSlice::from_string("the little brown fox");
        assert_eq!(ts[1], Token::alpha("little".into(), 4, 10, 1, 6))
    }

    #[test]
    fn test_is_empty() {
        let ts = TokenSlice::from_string("");
        assert!(ts.is_empty());
    }

    #[test]
    fn test_scan_to() {
        let mut ts = TokenSlice::from_string("the fox was too 'fast!' for me");
        assert_eq!(ts.scan_to(|t| t.is_single_quoted()), [
            Token::alpha("the".into(), 0, 3, 1, 2),
            Token::alpha("fox".into(), 4, 7, 1, 6),
            Token::alpha("was".into(), 8, 11, 1, 10),
            Token::alpha("too".into(), 12, 15, 1, 14)
        ]);
    }

    #[test]
    fn test_scan_until() {
        let mut ts = TokenSlice::from_string("the fox was too 'fast!' for me");
        assert_eq!(ts.scan_until(|t| t.is_single_quoted()), [
            Token::alpha("the".into(), 0, 3, 1, 2),
            Token::alpha("fox".into(), 4, 7, 1, 6),
            Token::alpha("was".into(), 8, 11, 1, 10),
            Token::alpha("too".into(), 12, 15, 1, 14),
            Token::single_quoted("'fast!'".into(), 16, 23, 1, 18)
        ]);
    }
}