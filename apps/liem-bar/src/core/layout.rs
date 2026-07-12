use crate::core::config::LayoutNode;

#[derive(Debug, Clone)]
pub struct PositionedWidget {
    pub widget_id: String,
    pub bounds_x: f32,
    pub bounds_y: f32,
    pub bounds_w: f32,
    pub bounds_h: f32,
}

pub fn evaluate_layout(
    node: &LayoutNode,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Vec<PositionedWidget> {
    let mut widgets = Vec::new();
    evaluate_node(node, x, y, w, h, &mut widgets);
    widgets
}

fn evaluate_node(
    node: &LayoutNode,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    out: &mut Vec<PositionedWidget>,
) {
    match node {
        LayoutNode::Widget { widget_id } => {
            out.push(PositionedWidget {
                widget_id: widget_id.clone(),
                bounds_x: x,
                bounds_y: y,
                bounds_w: w,
                bounds_h: h,
            });
        }
        LayoutNode::Spacer => {}
        LayoutNode::Row { children } => {
            if children.is_empty() {
                return;
            }
            let child_w = w / children.len() as f32;
            for (i, child) in children.iter().enumerate() {
                let child_x = x + (i as f32 * child_w);
                evaluate_node(child, child_x, y, child_w, h, out);
            }
        }
        LayoutNode::Column { children } => {
            if children.is_empty() {
                return;
            }
            let child_h = h / children.len() as f32;
            for (i, child) in children.iter().enumerate() {
                let child_y = y + (i as f32 * child_h);
                evaluate_node(child, x, child_y, w, child_h, out);
            }
        }
        LayoutNode::Group { children } => {
            for child in children {
                evaluate_node(child, x, y, w, h, out);
            }
        }
    }
}
