//! Raft consensus node wrapper.

use crate::common::Result;
use crate::coordinator::raft_rpc_client::{send_append_entries_rpc, send_request_vote_rpc};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

impl std::fmt::Display for RaftRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaftRole::Follower => write!(f, "follower"),
            RaftRole::Candidate => write!(f, "candidate"),
            RaftRole::Leader => write!(f, "leader"),
        }
    }
}

pub struct RaftNode {
    node_id: String,
    role: Arc<Mutex<RaftRole>>,
    term: Arc<Mutex<u64>>,
    voted_for: Arc<Mutex<Option<String>>>,
    leader_id: Arc<Mutex<Option<String>>>,
    log: Arc<Mutex<Vec<crate::common::raft::LogEntry>>>,
    peers: Arc<Mutex<Vec<String>>>, // List of peer node IDs
    commit_index: Arc<Mutex<u64>>,
    last_applied: Arc<Mutex<u64>>,
    snapshot: Arc<Mutex<Option<Vec<u8>>>>, // Optionally store snapshot bytes
}

impl RaftNode {
    pub fn get_peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().clone()
    }
    pub fn detect_partition(
        &self,
        last_heartbeat: tokio::time::Instant,
        timeout: tokio::time::Duration,
    ) -> bool {
        last_heartbeat.elapsed() > timeout
    }

    pub fn recover(&self) {
        let _ = self.load_snapshot().is_some();
        let _ = self.log.lock().unwrap().clone();
        let mut applied = self.last_applied.lock().unwrap();
        *applied = *self.commit_index.lock().unwrap();
    }
    pub fn save_snapshot(&self, data: Vec<u8>) {
        let mut snap = self.snapshot.lock().unwrap();
        *snap = Some(data);
    }

    pub fn load_snapshot(&self) -> Option<Vec<u8>> {
        self.snapshot.lock().unwrap().clone()
    }

    pub fn apply_snapshot(&self, data: Vec<u8>, last_included_index: u64, last_included_term: u64) {
        self.save_snapshot(data);
        let mut log = self.log.lock().unwrap();
        log.retain(|entry| entry.index > last_included_index);
        let mut commit = self.commit_index.lock().unwrap();
        *commit = last_included_index;
        let mut applied = self.last_applied.lock().unwrap();
        *applied = last_included_index;
        let mut term = self.term.lock().unwrap();
        *term = last_included_term;
    }
    pub fn get_log(&self) -> std::sync::MutexGuard<'_, Vec<crate::common::raft::LogEntry>> {
        self.log.lock().unwrap()
    }

    pub async fn send_heartbeats(&self) {
        let peers = self.peers.lock().unwrap().clone();
        let term = self.get_term();
        let leader_id = self.node_id.clone();
        let log_snapshot = self.log.lock().unwrap().clone();
        let prev_log_index = log_snapshot.last().map(|e| e.index).unwrap_or(0);
        let prev_log_term = log_snapshot.last().map(|e| e.term).unwrap_or(0);
        let leader_commit = prev_log_index;
        for peer in &peers {
            let req = crate::common::raft::AppendRequest {
                term,
                leader_id: leader_id.clone(),
                prev_log_index,
                prev_log_term,
                entries: vec![], // Heartbeat: no entries
                leader_commit,
            };
            let _ = send_append_entries_rpc(peer, req).await;
        }
    }

    /// Start the election timer and trigger elections if no heartbeat is received.
    pub async fn run_election_timer(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        loop {
            let timeout = rng.gen_range(150..300);
            tokio::time::sleep(tokio::time::Duration::from_millis(timeout)).await;
            if !self.is_leader() {
                let peers = {
                    let peers_guard = self.peers.lock().unwrap();
                    peers_guard.clone()
                };
                self.start_election_and_collect_votes(peers).await;
            }
        }
    }

    pub fn handle_request_vote(
        &self,
        req: crate::common::raft::VoteRequest,
    ) -> crate::common::raft::VoteResponse {
        let mut term = self.term.lock().unwrap();
        let mut voted_for = self.voted_for.lock().unwrap();
