use spotifast::addons::Runtime;

#[test]
fn visual_history_recycles_buffers_and_keeps_the_latest_frames_in_order() {
    let mut runtime = Runtime::default();
    for frame in 0..24 {
        runtime.update_visual_levels(&[frame as f32; 64], true);
    }
    let bands = runtime.bands.as_ptr();
    let allocations: std::collections::HashSet<_> =
        runtime.ridges.iter().map(|v| v.as_ptr()).collect();
    for frame in 24..240 {
        runtime.update_visual_levels(&[frame as f32; 64], true);
        assert_eq!(runtime.bands.as_ptr(), bands);
        assert_eq!(runtime.ridges.len(), 24);
        assert!(
            runtime
                .ridges
                .iter()
                .all(|v| allocations.contains(&v.as_ptr()))
        );
        for (age, ridge) in runtime.ridges.iter().enumerate() {
            assert_eq!(ridge, &vec![(frame - age) as f32; 64]);
        }
    }
}

#[test]
fn current_only_visuals_and_pauses_do_not_build_history() {
    let mut runtime = Runtime::default();
    runtime.update_visual_levels(&[0.5; 64], false);
    assert!(runtime.ridges.is_empty());
    assert_eq!(runtime.bands, vec![0.5; 64]);
    runtime.update_visual_levels(&[0.7; 64], true);
    runtime.update_visual_levels(&[0.0; 64], false);
    assert_eq!(runtime.ridges.len(), 1);
    assert_eq!(runtime.ridges[0], vec![0.7; 64]);
    assert_eq!(runtime.bands, vec![0.0; 64]);
}
