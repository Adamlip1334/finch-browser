//! Basic CSS block layout.

use crate::style::{StyledNode, Display};
use crate::css::Value::{Keyword, Length};
use crate::css::Unit::Px;
use std::default::Default;

pub use self::BoxType::{AnonymousBlock, InlineNode, BlockNode};

// CSS box model. all sizes are in px

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    pub content: Rect,

    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

// a node in the layout tree.
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) | InlineNode(node) => node,
            AnonymousBlock => panic!("Anonymous block box has no style node")
        }
    }
}


pub fn layout_tree<'a>(node: &'a StyledNode<'a>, mut containing_block: Dimensions) -> LayoutBox<'a> {
    // The layout algorithm expects the container height to start at 0
    // TODO: Save the initial containing block height, for calculating percent heights.
    containing_block.content.height = 0.0;

    let mut root_box = build_layout_tree(node);
    root_box.layout(containing_block);
    root_box
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node),
        Display::Inline => InlineNode(style_node),
        Display::None => panic!("Root node has display: none.")
    });

    // Create the descendant boxes.
    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root.get_inline_container().children.push(build_layout_tree(child)),
            Display::None => {} // Don't lay out nodes with `display: none;`
        }
    }
    root
}

impl<'a> LayoutBox<'a> {
    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) | AnonymousBlock => {} // TODO
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        self.calculate_block_position(containing_block);

        self.layout_block_children();

        self.calculate_block_height();
    }

    /// http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0
        let zero = Length(0.0, Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = sum([&margin_left, &margin_right, &border_left, &border_right,
                         &padding_left, &padding_right, &width].iter().map(|v| v.to_px()));

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }
            if margin_right == auto {
                margin_right = Length(0.0, Px);
            }
        }

        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false, false, false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }

            (false, false, true) => { margin_right = Length(underflow, Px); }
            (false, true, false) => { margin_left  = Length(underflow, Px); }

            (true, _, _) => {
                if margin_left == auto { margin_left = Length(0.0, Px); }
                if margin_right == auto { margin_right = Length(0.0, Px); }

                if underflow >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Length(underflow, Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = Length(0.0, Px);
                    margin_right = Length(margin_right.to_px() + underflow, Px);
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style.lookup("border-top-width", "border-width", &zero).to_px();
        d.border.bottom = style.lookup("border-bottom-width", "border-width", &zero).to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.x +
                      d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + containing_block.content.y +
                      d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the block's children within its content area.
    ///
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(*d);
            // Increment the height so each child is laid out below the previous one.
            d.content.height = d.content.height + child.dimensions.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox { box_type: AnonymousBlock,..}) => {}
                    _ => self.children.push(LayoutBox::new(AnonymousBlock))
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}

impl Rect {
    pub fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl Dimensions {
    pub fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    pub fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    pub fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
