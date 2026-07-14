//! LocalAgreement-2: commits the token prefix that stays stable across two consecutive
//! hypotheses. This is the classic "local agreement" streaming-commit strategy used by
//! streaming ASR systems to avoid re-emitting text that later gets revised.

use super::{Committed, StreamPolicy, Token};

/// Commits tokens that are stable across the last two observed hypotheses.
#[derive(Debug, Clone, Default)]
pub struct LocalAgreement2 {
    previous: Option<Vec<Token>>,
    committed_upto: usize,
}

impl LocalAgreement2 {
    pub fn new() -> Self {
        Self {
            previous: None,
            committed_upto: 0,
        }
    }
}

/// Length of the common prefix of `a` and `b`, comparing tokens by `text`.
fn common_prefix_len(a: &[Token], b: &[Token]) -> usize {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x.text == y.text)
        .count()
}

impl StreamPolicy for LocalAgreement2 {
    fn observe(&mut self, hypothesis: &[Token]) -> Committed {
        let result = match &self.previous {
            None => Committed {
                text: String::new(),
                committed_upto: self.committed_upto,
            },
            Some(previous) => {
                let common = common_prefix_len(previous, hypothesis);
                if common > self.committed_upto {
                    let newly = &hypothesis[self.committed_upto..common];
                    let text = newly
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    self.committed_upto = common;
                    Committed {
                        text,
                        committed_upto: self.committed_upto,
                    }
                } else {
                    Committed {
                        text: String::new(),
                        committed_upto: self.committed_upto,
                    }
                }
            }
        };
        self.previous = Some(hypothesis.to_vec());
        result
    }
}
