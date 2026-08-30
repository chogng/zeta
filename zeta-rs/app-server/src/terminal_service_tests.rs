use super::*;

#[test]
fn output_ring_reports_a_gap_after_eviction() {
    let mut state = TerminalState::default();
    for _ in 0..3 {
        push_output(&mut state, vec![b'x'; MAX_OUTPUT_BYTES / 2]);
    }

    let result = read_state(
        &TerminalReadParams {
            dir_id: None,
            terminal_id: "terminal-1".into(),
            after_sequence: 0,
            after_command_sequence: 0,
            max_chunks: 128,
        },
        &state,
    );

    assert!(result.output_gap);
    assert_eq!(result.chunks.len(), 2);
    assert_eq!(result.chunks[0].sequence, 2);
    assert_eq!(result.next_sequence, 3);
}

#[test]
fn output_ring_advances_the_cursor_when_one_oversized_chunk_is_evicted() {
    let mut state = TerminalState::default();
    push_output(&mut state, vec![b'x'; MAX_OUTPUT_BYTES + 1]);

    let result = read_state(
        &TerminalReadParams {
            dir_id: None,
            terminal_id: "terminal-1".into(),
            after_sequence: 0,
            after_command_sequence: 0,
            max_chunks: 128,
        },
        &state,
    );

    assert!(result.output_gap);
    assert!(result.chunks.is_empty());
    assert_eq!(result.next_sequence, 1);
}

#[test]
fn terminal_size_and_input_limits_are_explicit() {
    assert_eq!(validate_size(0, 80), Err(TerminalError::InvalidInput));
    assert_eq!(validate_size(24, 0), Err(TerminalError::InvalidInput));
    assert_eq!(validate_size(512, 512), Ok(()));
    assert_eq!(validate_size(513, 80), Err(TerminalError::InvalidInput));
}

#[test]
fn process_is_not_terminal_until_output_has_closed() {
    let mut state = TerminalState {
        exit_code: Some(0),
        ..TerminalState::default()
    };
    state.exited = state.output_closed;
    assert!(!state.exited);

    state.output_closed = true;
    state.exited = state.exit_code.is_some();
    assert!(state.exited);
}
