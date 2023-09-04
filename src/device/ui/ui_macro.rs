macro_rules! m_chain {
    (
        $first:expr,
        $($element:expr),*  $(,)?
    ) => {
        embedded_layout::prelude::Chain::new($first)
        $(.append(
            $element
        ))*
    };
}
pub(crate) use m_chain;


macro_rules! chain_text {
      (
        $buf:expr, $style:expr,
        $first_range:expr,
        $($range:expr),*
    ) => {
          m_chain!(
              embedded_graphics::text::Text::new(&$buf[$first_range], embedded_graphics_core::prelude::Point::zero(), $style.clone()),
              $(
                  embedded_graphics::text::Text::new(&$buf[$range], embedded_graphics_core::prelude::Point::zero(), $style.clone()),
              )*
          )
    };
}

pub(crate) use chain_text;

macro_rules! chain_text_step {
    (
        $buf:expr, $style:expr,
        step=$step:expr, $($index:expr),*
    ) => {
        $crate::common::device::ui::ui_macro::chain_text!(
            $buf, $style,
            $($index..$index+$step),*
        )
    };
}

pub(crate) use chain_text_step;


macro_rules! inner_layout {
     (
        $typ:ident,
        $first_element:expr
        $(; spacing = $spacing:expr)?
        $(; alignment = $alignment:expr)?
    ) => {
        embedded_layout::layout::linear::LinearLayout::$typ($first_element)
        $(.with_spacing($spacing))?
        $(.with_alignment($alignment))?
         .arrange()
    };

    (
        $typ:ident,
        $first_element:expr,
        $($element:expr),*
        $(; spacing = $spacing:expr)?
        $(; alignment = $alignment:expr)?
    ) => {
        embedded_layout::layout::linear::LinearLayout::$typ(
            m_chain!($first_element, $($element),*)
        )
        $(.with_spacing($spacing))?
        $(.with_alignment($alignment))?
        .arrange()
    };
}

pub(crate) use inner_layout;

macro_rules! v_layout {
    (
        $($token:tt)*
    ) => {
        $crate::common::device::ui::ui_macro::inner_layout!(vertical, $($token)*)
    };
}

pub(crate) use v_layout;


macro_rules! h_layout {
    (
        $($token:tt)*
    ) => {
        $crate::common::device::ui::ui_macro::inner_layout!(horizontal, $($token)*)
    };
}

pub(crate) use h_layout;
