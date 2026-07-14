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
        Self { previous: None, committed_upto: 0 }
    }
}

/// Length of the common prefix of `a` and `b`, comparing tokens by `text`.
fn common_prefix_len(a: &[Token], b: &[Token]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x.text == y.text).count()
}

impl StreamPolicy for LocalAgreement2 {
    fn observe(&mut self, hypothesis: &[Token]) -> Committed {
        let from = self.committed_upto;
        let result = match &self.previous {
            None => Committed { text: String::new(), committed_upto: self.committed_upto, committed_from: from },
            Some(previous) => {
                let common = common_prefix_len(previous, hypothesis);
                if common > self.committed_upto {
                    let newly = &hypothesis[self.committed_upto..common];
                    let text = super::join_tokens(newly);
                    self.committed_upto = common;
                    Committed { text, committed_upto: self.committed_upto, committed_from: from }
                } else {
                    Committed { text: String::new(), committed_upto: self.committed_upto, committed_from: from }
                }
            }
        };
        self.previous = Some(hypothesis.to_vec());
        result
    }

    /// Final signal: commit the tail from the cursor to the end of `hypothesis`. Idempotent —
    /// re-committing after the cursor has reached the end yields empty text.
    fn observe_final(&mut self, hypothesis: &[Token]) -> Committed {
        let from = self.committed_upto;
        let newly = if hypothesis.len() > self.committed_upto { &hypothesis[self.committed_upto..] } else { &[] };
        let text = super::join_tokens(newly);
        self.committed_upto = hypothesis.len().max(self.committed_upto);
        self.previous = Some(hypothesis.to_vec());
        Committed { text, committed_upto: self.committed_upto, committed_from: from }
    }
}
