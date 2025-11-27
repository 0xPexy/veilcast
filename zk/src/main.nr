use dep::std::hash::pedersen_hash;

/// VeilCast vote reveal circuit (commit-reveal + nullifier).
/// Constraints:
///  - commitment = Poseidon(choice, secret)
///  - nullifier  = Poseidon(identity_secret, poll_id)
/// Public inputs: commitment, nullifier, poll_id.
/// Private inputs: choice, secret, identity_secret.
fn main(
    commitment: pub Field,
    nullifier: pub Field,
    poll_id: pub Field,
    choice: Field,
    secret: Field,
    identity_secret: Field,
) {
    // Enforce choice is 0/1
    assert(choice * (1 - choice) == 0);

    // Commitment check
    let computed_commitment = pedersen_hash([choice, secret]);
    assert(computed_commitment == commitment);

    // Nullifier ties an identity_secret to poll_id to enforce 1 vote per poll.
    let computed_nullifier = pedersen_hash([identity_secret, poll_id]);
    assert(computed_nullifier == nullifier);
}
