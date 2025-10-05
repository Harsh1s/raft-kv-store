impl From<&VoteRequest> for crate::proto::VoteRequest {
    fn from(req: &VoteRequest) -> Self {
        Self {
            term: req.term,
            candidate_id: req.candidate_id.clone(),
            last_log_index: req.last_log_index,
            last_log_term: req.last_log_term,
        }
    }
}

impl From<&crate::proto::VoteResponse> for VoteResponse {
    fn from(resp: &crate::proto::VoteResponse) -> Self {
        Self {
            term: resp.term,
            vote_granted: resp.vote_granted,
        }
    }
}

impl From<&AppendRequest> for crate::proto::AppendRequest {
    fn from(req: &AppendRequest) -> Self {
        Self {
            term: req.term,
            leader_id: req.leader_id.clone(),
            prev_log_index: req.prev_log_index,
            prev_log_term: req.prev_log_term,
            entries: req.entries.iter().map(|e| e.into()).collect(),
            leader_commit: req.leader_commit,
        }
