use spotifast::window::{MAIN_MIN_SIZE, recover_main_size, usable_size};

fn frame(
    ctx: &egui::Context,
    size: Option<[f32; 2]>,
    minimized: bool,
    fullscreen: bool,
) -> Vec<egui::ViewportCommand> {
    let mut input = egui::RawInput::default();
    let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.inner_rect =
        size.map(|size| egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0], size[1])));
    viewport.minimized = Some(minimized);
    viewport.fullscreen = Some(fullscreen);
    let output = ctx.run_ui(input, |_| recover_main_size(ctx, Some([1100.0, 700.0])));
    output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .clone()
}

#[test]
fn tiny_framework_restore_recovers_remembered_size_only_once() {
    let ctx = egui::Context::default();
    let commands = frame(&ctx, Some([275.0, 116.0]), false, false);
    assert!(commands.iter().any(|c| matches!(c, egui::ViewportCommand::InnerSize(size) if *size == egui::vec2(1100.0, 700.0))));
    assert!(frame(&ctx, Some([275.0, 116.0]), false, false).is_empty());
}

#[test]
fn recovery_waits_until_a_visible_viewport_is_known() {
    let ctx = egui::Context::default();
    assert!(frame(&ctx, None, false, false).is_empty());
    assert!(frame(&ctx, Some([1.0, 1.0]), true, false).is_empty());
    assert!(!frame(&ctx, Some([275.0, 116.0]), false, false).is_empty());
}

#[test]
fn valid_layout_and_portrait_fullscreen_are_preserved() {
    assert!(
        frame(
            &egui::Context::default(),
            Some([940.0, 680.0]),
            false,
            false
        )
        .is_empty()
    );
    assert!(
        frame(
            &egui::Context::default(),
            Some([600.0, 1000.0]),
            false,
            true
        )
        .is_empty()
    );
}

#[test]
fn invalid_saved_dimensions_are_never_accepted() {
    for size in [[f32::INFINITY, 800.0], [1240.0, f32::NAN], [275.0, 116.0]] {
        assert_eq!(usable_size(Some(size)), None);
    }
    assert_eq!(usable_size(Some(MAIN_MIN_SIZE)), Some(MAIN_MIN_SIZE));
}

#[test]
fn interface_zoom_does_not_make_a_valid_native_window_look_too_small() {
    let ctx = egui::Context::default();
    ctx.set_zoom_factor(1.5);
    let _ = ctx.run_ui(Default::default(), |_| {});
    assert!(frame(&ctx, Some([940.0 / 1.5, 680.0 / 1.5]), false, false).is_empty());
}
