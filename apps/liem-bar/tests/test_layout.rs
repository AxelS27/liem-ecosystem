use liem_bar::core::config::LayoutNode;
use liem_bar::core::layout::evaluate_layout;

#[test]
fn test_row_spacer_widget_layout() {
    let layout = LayoutNode::Row {
        children: vec![
            LayoutNode::Widget { widget_id: "clock.time".to_string() },
            LayoutNode::Spacer,
            LayoutNode::Widget { widget_id: "clock.date".to_string() },
        ],
    };

    let positioned = evaluate_layout(&layout, 0.0, 0.0, 1200.0, 40.0);

    // There should be 2 positioned widgets (Spacers don't render content)
    assert_eq!(positioned.len(), 2);

    // Each child (including spacer) takes 1200 / 3 = 400 pixels
    let clock_time = &positioned[0];
    assert_eq!(clock_time.widget_id, "clock.time");
    assert_eq!(clock_time.bounds_x, 0.0);
    assert_eq!(clock_time.bounds_w, 400.0);
    assert_eq!(clock_time.bounds_h, 40.0);

    let clock_date = &positioned[1];
    assert_eq!(clock_date.widget_id, "clock.date");
    assert_eq!(clock_date.bounds_x, 800.0);
    assert_eq!(clock_date.bounds_w, 400.0);
}
