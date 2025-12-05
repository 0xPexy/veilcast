export type Phase = 'commit' | 'reveal' | 'resolved';

export interface Poll {
  id: number;
  question: string;
  options: string[];
  commit_phase_end: string;
  reveal_phase_end: string;
  membership_root: string;
  owner: string;
  reveal_tx_hash?: string;
  correct_option?: number | null;
  resolved: boolean;
  category: string;
  commit_sync_completed: boolean;
  vote_counts: number[];
}

export interface PollView extends Poll {
  phase: Phase;
  countdown: string;
}

export interface UserStats {
  username: string;
  tier: string;
  xp: number;
  total_votes: number;
  correct_votes: number;
  accuracy: number;
  rank?: number;
}

export interface CreatePollResult {
  poll: Poll;
  tx_hash: string;
}

export interface MembershipStatus {
  poll_id: number;
  membership_root: string;
  is_member: boolean;
  path_bits?: string[];
  path_siblings?: string[];
}

export interface CommitStatus {
  poll_id: number;
  already_committed: boolean;
}

export interface GeneratedProof {
  commitment: string;
  nullifier: string;
  proof: string;
  public_inputs: string[];
}
