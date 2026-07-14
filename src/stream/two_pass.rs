//! Two-pass streaming policy: tentative hypotheses are buffered and emit nothing; committing
//! only happens on an explicit final signal (e.g. end of utterance / VAD silence boundary),
//! which commits everything not yet committed.

use super::{Committed, StreamPolicy, Token};

/// Buffers the latest tentative hypothesis; commits only on an explicit final signal.
#[derive(Debug, Clone, Default)]
pub struct TwoPass {
    latest: Vec<Token>,
    committed_upto: usize,
}

impl TwoPass {
    pub fn new() -> Self {
        Self { latest: Vec::new(), committed_upto: 0 }
    }
}

impl StreamPolicy for TwoPass {
    fn observe(&mut self, hypothesis: &[Token]) -> Committed {
        self.latest = hypothesis.to_vec();
        Committed { text: String::new(), committed_upto: self.committed_upto, committed_from: self.committed_upto }
    }

    /// Commit everything in `hypothesis` beyond what has already been committed.
    fn observe_final(&mut self, hypothesis: &[Token]) -> Committed {
        let from = self.committed_upto;
        let newly = if hypothesis.len() > self.committed_upto { &hypothesis[self.committed_upto..] } else { &[] };
        let text = super::join_tokens(newly);
        self.committed_upto = hypothesis.len().max(self.committed_upto);
        self.latest = hypothesis.to_vec();
        Committed { text, committed_upto: self.committed_upto, committed_from: from }
    }
}
