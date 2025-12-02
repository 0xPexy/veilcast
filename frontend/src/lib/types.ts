export type Phase = 'commit' | 'reveal' | 'resolved';

export interface Poll {
  id: number;
  question: string;
  options: string[];
  commit_phase_end: string;
  reveal_phase_end: string;
  membership_root: string;
  correct_option?: number | null;
  resolved: boolean;
  category: string;
}

export interface PollView extends Poll {
  phase: Phase;
  countdown: string;
}

export interface MembershipStatus {
  poll_id: number;
  membership_root: string;
  is_member: boolean;
}

export interface CommitStatus {
  poll_id: number;
  already_committed: boolean;
}
