use dep::poseidon::poseidon2_permutation;

fn poseidon2_hash1(x: Field) -> Field {
    // Pad to width 4 to align with barretenberg poseidon2 arity
    poseidon2_permutation([x, 0, 0, 0], 4)[0]
}

fn poseidon2_hash2(a: Field, b: Field) -> Field {
    poseidon2_permutation([a, b, 0, 0], 4)[0]
}

/// VeilCast vote reveal circuit (commit-reveal + nullifier).
/// Constraints:
///  - commitment = Poseidon(choice, secret)
///  - nullifier  = Poseidon(identity_secret, poll_id)
/// Public inputs: commitment, nullifier, poll_id.
/// Private inputs: choice, secret, identity_secret, merkle path (siblings + direction bits) proving group membership.
fn main(
    commitment: pub Field,
    nullifier: pub Field,
    poll_id: pub Field,
    membership_root: pub Field,
    choice: Field,
    secret: Field,
    identity_secret: Field,
    path_siblings: [Field; 20],
    path_bits: [Field; 20],
) {
    // Enforce choice is 0/1
    assert(choice * (1 - choice) == 0);

    // Merkle membership: leaf = hash_1([identity_secret]), path defined by path_bits (0 left, 1 right)
    let mut node = poseidon2_hash1(identity_secret);
    for i in 0..20 {
        let bit = path_bits[i];
        assert(bit * (1 - bit) == 0); // boolean
        let sib = path_siblings[i];
        node = if bit == 0 { poseidon2_hash2(node, sib) } else { poseidon2_hash2(sib, node) };
    }
    assert(node == membership_root);

    // Commitment check
    let computed_commitment = poseidon2_hash2(choice, secret);
    assert(computed_commitment == commitment);

    // Nullifier ties an identity_secret to poll_id to enforce 1 vote per poll.
    let computed_nullifier = poseidon2_hash2(identity_secret, poll_id);
    assert(computed_nullifier == nullifier);
}

fn compute_root(identity_secret: Field, path_siblings: [Field; 20], path_bits: [Field; 20]) -> Field {
    let mut node = poseidon2_hash1(identity_secret);
    for i in 0..20 {
        let bit = path_bits[i];
        assert(bit * (1 - bit) == 0); // boolean
        let sib = path_siblings[i];
        node = if bit == 0 { poseidon2_hash2(node, sib) } else { poseidon2_hash2(sib, node) };
    }
    node
}

#[test]
fn test_valid_membership_and_commitment() {
    let choice = 1;
    let secret = 42;
    let identity_secret = 123;
    let poll_id = 7;

    // All-zero path (depth 20). Adjust in real use to match actual tree.
    let path_siblings: [Field; 20] = [0; 20];
    let path_bits: [Field; 20] = [0; 20];

    // Derive membership root for the zero-path.
    let membership_root = compute_root(identity_secret, path_siblings, path_bits);

    // Commitments and nullifier
    let commitment = poseidon2_hash2(choice, secret);
    let nullifier = poseidon2_hash2(identity_secret, poll_id);

    // Should not assert
    main(
        commitment,
        nullifier,
        poll_id,
        membership_root,
        choice,
        secret,
        identity_secret,
        path_siblings,
        path_bits,
    );
}

#[test(should_fail)]
fn test_invalid_choice_non_binary() {
    let choice = 2;
    let secret = 42;
    let identity_secret = 123;
    let poll_id = 7;
    let path_siblings: [Field; 20] = [0; 20];
    let path_bits: [Field; 20] = [0; 20];
    let membership_root = compute_root(identity_secret, path_siblings, path_bits);

    let commitment = poseidon2_hash2(choice, secret);
    let nullifier = poseidon2_hash2(identity_secret, poll_id);

    main(
        commitment,
        nullifier,
        poll_id,
        membership_root,
        choice,
        secret,
        identity_secret,
        path_siblings,
        path_bits,
    );
}

#[test(should_fail)]
fn test_wrong_commitment() {
    let choice = 1;
    let secret = 42;
    let bad_secret = 99;
    let identity_secret = 123;
    let poll_id = 7;
    let path_siblings: [Field; 20] = [0; 20];
    let path_bits: [Field; 20] = [0; 20];
    let membership_root = compute_root(identity_secret, path_siblings, path_bits);

    // Commitment does not match provided secret
    let commitment = poseidon2_hash2(choice, bad_secret);
    let nullifier = poseidon2_hash2(identity_secret, poll_id);

    main(
        commitment,
        nullifier,
        poll_id,
        membership_root,
        choice,
        secret,
        identity_secret,
        path_siblings,
        path_bits,
    );
}

#[test(should_fail)]
fn test_wrong_merkle_path() {
    let choice = 1;
    let secret = 42;
    let identity_secret = 123;
    let poll_id = 7;
    let mut path_siblings: [Field; 20] = [0; 20];
    let mut path_bits: [Field; 20] = [0; 20];
    // Introduce wrong sibling at level 0
    path_siblings[0] = 999;

    let correct_root = compute_root(identity_secret, [0; 20], [0; 20]);
    let commitment = poseidon2_hash2(choice, secret);
    let nullifier = poseidon2_hash2(identity_secret, poll_id);

    main(
        commitment,
        nullifier,
        poll_id,
        correct_root,
        choice,
        secret,
        identity_secret,
        path_siblings,
        path_bits,
    );
}

#[test(should_fail)]
fn test_wrong_nullifier_poll_id() {
    let choice = 1;
    let secret = 42;
    let identity_secret = 123;
    let poll_id = 7;
    let bad_poll_id = 8;
    let path_siblings: [Field; 20] = [0; 20];
    let path_bits: [Field; 20] = [0; 20];
    let membership_root = compute_root(identity_secret, path_siblings, path_bits);

    let commitment = poseidon2_hash2(choice, secret);
    // Nullifier computed with wrong poll id
    let nullifier = poseidon2_hash2(identity_secret, bad_poll_id);

    main(
        commitment,
        nullifier,
        poll_id,
        membership_root,
        choice,
        secret,
        identity_secret,
        path_siblings,
        path_bits,
    );
}
